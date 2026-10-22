//! https://learn.microsoft.com/windows/win32/api/mmdeviceapi/nn-mmdeviceapi-immnotificationclient
use super::*;
use std::sync::mpsc::Receiver;
use windows::{
    core::{implement, PCWSTR, PWSTR},
    Win32::{
        Devices::FunctionDiscovery::{PKEY_Device_EnumeratorName, PKEY_Device_FriendlyName},
        Foundation::PROPERTYKEY,
        Media::Audio::*,
        Storage::EnhancedStorage::PKEY_Devices_AudioDevice_RawProcessingSupported,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize,
                StructuredStorage::{
                    PropVariantClear, PropVariantToBoolean, PropVariantToStringAlloc,
                    PropVariantToUInt32, PROPVARIANT,
                },
                CLSCTX_ALL, COINIT_MULTITHREADED, STGM_READ,
            },
            Power::{GetSystemPowerStatus, SYSTEM_POWER_STATUS},
        },
        UI::Shell::PropertiesSystem::IPropertyStore,
    },
};

struct Apartment;
impl Apartment {
    fn enter() -> windows::core::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED).ok()?;
        }
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

#[implement(IMMNotificationClient)]
struct Notifications {
    wake: SyncSender<()>,
}
impl Notifications_Impl {
    fn changed(&self) -> windows::core::Result<()> {
        let _ = self.wake.try_send(());
        Ok(())
    }
}
impl IMMNotificationClient_Impl for Notifications_Impl {
    fn OnDeviceStateChanged(&self, _: &PCWSTR, _: DEVICE_STATE) -> windows::core::Result<()> {
        self.changed()
    }
    fn OnDeviceAdded(&self, _: &PCWSTR) -> windows::core::Result<()> {
        self.changed()
    }
    fn OnDeviceRemoved(&self, _: &PCWSTR) -> windows::core::Result<()> {
        self.changed()
    }
    fn OnDefaultDeviceChanged(
        &self,
        _: EDataFlow,
        _: ERole,
        _: &PCWSTR,
    ) -> windows::core::Result<()> {
        self.changed()
    }
    fn OnPropertyValueChanged(&self, _: &PCWSTR, _: &PROPERTYKEY) -> windows::core::Result<()> {
        self.changed()
    }
}
struct Registration {
    enumerator: IMMDeviceEnumerator,
    notifications: IMMNotificationClient,
}
impl Drop for Registration {
    fn drop(&mut self) {
        unsafe {
            let _ = self
                .enumerator
                .UnregisterEndpointNotificationCallback(&self.notifications);
        }
    }
}
struct Property(PROPVARIANT);
impl Drop for Property {
    fn drop(&mut self) {
        unsafe {
            let _ = PropVariantClear(&mut self.0);
        }
    }
}
fn property(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<Property> {
    unsafe { store.GetValue(key).ok().map(Property) }
}
fn text(store: &IPropertyStore, key: &PROPERTYKEY) -> Option<String> {
    let value = property(store, key)?;
    unsafe { take_string(PropVariantToStringAlloc(&value.0).ok()?).ok() }
}
unsafe fn take_string(value: PWSTR) -> windows::core::Result<String> {
    let text = value.to_string();
    CoTaskMemFree(Some(value.0.cast()));
    Ok(text?)
}
fn default_id(enumerator: &IMMDeviceEnumerator, role: EDataFlow) -> Option<String> {
    unsafe {
        take_string(
            enumerator
                .GetDefaultAudioEndpoint(role, eMultimedia)
                .ok()?
                .GetId()
                .ok()?,
        )
        .ok()
    }
}
fn adapter_enumerator(enumerator: &IMMDeviceEnumerator, endpoint: &IMMDevice) -> Option<String> {
    // The endpoint itself is a software device. Ask its connected adapter for bus metadata.
    // https://learn.microsoft.com/windows/win32/api/devicetopology/nf-devicetopology-idevicetopology-getdeviceid
    unsafe {
        let topology: IDeviceTopology = endpoint.Activate(CLSCTX_ALL, None).ok()?;
        let adapter = take_string(
            topology
                .GetConnector(0)
                .ok()?
                .GetDeviceIdConnectedTo()
                .ok()?,
        )
        .ok()?;
        let adapter = windows::core::HSTRING::from(adapter);
        let adapter = enumerator.GetDevice(&adapter).ok()?;
        let store = adapter.OpenPropertyStore(STGM_READ).ok()?;
        text(&store, &PKEY_Device_EnumeratorName)
    }
}
fn endpoints(
    enumerator: &IMMDeviceEnumerator,
    role: EDataFlow,
) -> windows::core::Result<Vec<AudioDevice>> {
    unsafe {
        let collection = enumerator.EnumAudioEndpoints(role, DEVICE_STATE_ACTIVE)?;
        let mut devices = vec![];
        for index in 0..collection.GetCount()?.min(256) {
            let Ok(endpoint) = collection.Item(index) else {
                continue;
            };
            let Ok(id) = endpoint.GetId().and_then(|id| take_string(id)) else {
                continue;
            };
            let Ok(store) = endpoint.OpenPropertyStore(STGM_READ) else {
                continue;
            };
            let name = text(&store, &PKEY_Device_FriendlyName)
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| "Audio device".into());
            let factor = property(&store, &PKEY_AudioEndpoint_FormFactor)
                .and_then(|value| PropVariantToUInt32(&value.0).ok())
                .unwrap_or(u32::MAX);
            let raw_supported = property(&store, &PKEY_Devices_AudioDevice_RawProcessingSupported)
                .and_then(|value| PropVariantToBoolean(&value.0).ok())
                .is_some_and(|value| value.as_bool());
            let bus = adapter_enumerator(enumerator, &endpoint)
                .or_else(|| text(&store, &PKEY_Device_EnumeratorName));
            let (bluetooth, built_in) = transport(bus.as_deref());
            devices.push(AudioDevice {
                id,
                name,
                form_factor: form_factor(factor).into(),
                raw_supported,
                bluetooth,
                built_in,
            });
        }
        devices.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));
        Ok(devices)
    }
}
fn snapshot(
    enumerator: &IMMDeviceEnumerator,
    revision: u64,
) -> windows::core::Result<AudioDevices> {
    let mut power = SYSTEM_POWER_STATUS::default();
    let has_battery = unsafe {
        GetSystemPowerStatus(&mut power)
            .ok()
            .and_then(|_| (power.BatteryFlag != 255).then_some(power.BatteryFlag & 128 == 0))
    };
    let mut devices = AudioDevices {
        speakers: endpoints(enumerator, eRender)?,
        microphones: endpoints(enumerator, eCapture)?,
        default_speaker_id: default_id(enumerator, eRender),
        default_microphone_id: default_id(enumerator, eCapture),
        has_battery,
        recommended_source: "phone".into(),
        revision,
        error: None,
    };
    devices.recommend();
    Ok(devices)
}
pub(super) fn watch(
    view: watch::Sender<AudioDevices>,
    stop: Arc<AtomicBool>,
    wake: SyncSender<()>,
    notices: Receiver<()>,
) {
    let run = || -> windows::core::Result<()> {
        let _apartment = Apartment::enter()?;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)? };
        let notifications: IMMNotificationClient = Notifications { wake }.into();
        unsafe {
            enumerator.RegisterEndpointNotificationCallback(&notifications)?;
        }
        let _registration = Registration {
            enumerator: enumerator.clone(),
            notifications,
        };
        let mut revision = 0;
        loop {
            if stop.load(Ordering::Acquire) {
                break;
            }
            revision += 1;
            match snapshot(&enumerator, revision) {
                Ok(devices) => {
                    view.send_replace(devices);
                }
                Err(_) => {
                    view.send_replace(AudioDevices {
                        revision,
                        error: Some("device_list_unavailable".into()),
                        ..Default::default()
                    });
                }
            }
            // Callbacks only wake this owner; all COM work and destruction stay here.
            if stop.load(Ordering::Acquire) {
                return Ok(());
            }
            match notices.recv() {
                Ok(()) => while !stop.load(Ordering::Acquire) && notices.try_recv().is_ok() {},
                Err(_) => return Ok(()),
            }
        }
        Ok(())
    };
    if run().is_err() {
        view.send_replace(AudioDevices {
            error: Some("device_list_unavailable".into()),
            ..Default::default()
        });
    }
}
