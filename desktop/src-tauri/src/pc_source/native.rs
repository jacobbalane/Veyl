//! https://learn.microsoft.com/windows/win32/coreaudio/loopback-recording
use super::*;
use crate::pc_devices::AudioDevice;
use std::{mem::size_of, ptr::read_unaligned, time::Duration};
use windows::{
    core::{w, Interface, GUID, HSTRING},
    Win32::{
        Foundation::{CloseHandle, HANDLE, WAIT_FAILED},
        Media::Audio::*,
        System::{
            Com::{
                CoCreateInstance, CoInitializeEx, CoTaskMemFree, CoUninitialize, CLSCTX_ALL,
                COINIT_MULTITHREADED,
            },
            Threading::{
                AvRevertMmThreadCharacteristics, AvSetMmThreadCharacteristicsW, CreateEventW,
                WaitForSingleObject,
            },
        },
    },
};

struct Apartment;
impl Apartment {
    fn enter() -> io::Result<Self> {
        unsafe {
            CoInitializeEx(None, COINIT_MULTITHREADED)
                .ok()
                .map_err(error)?;
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
struct Event(HANDLE);
impl Drop for Event {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct Priority(HANDLE);
impl Drop for Priority {
    fn drop(&mut self) {
        unsafe {
            let _ = AvRevertMmThreadCharacteristics(self.0);
        }
    }
}
struct MixFormat(*mut WAVEFORMATEX);
impl Drop for MixFormat {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0.cast()));
        }
    }
}

pub(super) struct Wasapi {
    client: IAudioClient,
    capture: IAudioCaptureClient,
    event: Option<Event>,
    details: CaptureDetails,
    started: bool,
    _priority: Option<Priority>,
    // COM must remain initialized until all interfaces have been released.
    _apartment: Apartment,
}
impl Wasapi {
    pub fn open(device: &AudioDevice, stream: u8) -> io::Result<Box<dyn CaptureBackend>> {
        let apartment = Apartment::enter()?;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(error)? };
        let endpoint = unsafe {
            enumerator
                .GetDevice(&HSTRING::from(&device.id))
                .map_err(error)?
        };
        let raw = stream == 1 && device.raw_supported;
        let mut opened = None;
        let mut last = invalid("Could not initialize Windows audio.");
        for (raw_mode, event_driven) in [(raw, true), (raw, false), (false, true), (false, false)] {
            match initialize(&endpoint, stream, raw_mode, event_driven) {
                Ok(result) => {
                    opened = Some(result);
                    break;
                }
                Err(error) => {
                    if matches!(failure_code(&error), "device_invalidated" | "mic_silenced") {
                        return Err(error);
                    }
                    last = error;
                }
            }
        }
        let opened = opened.ok_or(last)?;
        let (client, capture, event, format, raw_mode, buffer_ms) = opened;
        let mut task = 0;
        let priority = unsafe {
            AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task)
                .ok()
                .map(Priority)
        };
        Ok(Box::new(Self {
            details: CaptureDetails {
                format,
                raw_mode,
                event_driven: event.is_some(),
                buffer_ms,
                pro_audio_thread: priority.is_some(),
            },
            client,
            capture,
            event,
            started: false,
            _priority: priority,
            _apartment: apartment,
        }))
    }
}
type Opened = (
    IAudioClient,
    IAudioCaptureClient,
    Option<Event>,
    CaptureFormat,
    bool,
    f64,
);
fn initialize(
    endpoint: &IMMDevice,
    stream: u8,
    want_raw: bool,
    event_driven: bool,
) -> io::Result<Opened> {
    unsafe {
        let client: IAudioClient = endpoint.Activate(CLSCTX_ALL, None).map_err(error)?;
        let mut raw_mode = false;
        if let Ok(client2) = client.cast::<IAudioClient2>() {
            let mut properties = AudioClientProperties {
                cbSize: size_of::<AudioClientProperties>() as u32,
                bIsOffload: false.into(),
                eCategory: AudioCategory_Other,
                Options: if want_raw {
                    AUDCLNT_STREAMOPTIONS_RAW
                } else {
                    AUDCLNT_STREAMOPTIONS_NONE
                },
            };
            if client2.SetClientProperties(&properties).is_ok() {
                raw_mode = want_raw;
            } else {
                properties.Options = AUDCLNT_STREAMOPTIONS_NONE;
                client2.SetClientProperties(&properties).map_err(error)?;
            }
        }
        let format = MixFormat(client.GetMixFormat().map_err(error)?);
        let decoded = decode_format(format.0)?;
        let event = if event_driven {
            Some(Event(
                CreateEventW(None, false, false, None).map_err(error)?,
            ))
        } else {
            None
        };
        let flags = if stream == 0 {
            AUDCLNT_STREAMFLAGS_LOOPBACK
        } else {
            0
        } | if event_driven {
            AUDCLNT_STREAMFLAGS_EVENTCALLBACK
        } else {
            0
        };
        client
            .Initialize(AUDCLNT_SHAREMODE_SHARED, flags, 200_000, 0, format.0, None)
            .map_err(error)?;
        if let Some(event) = &event {
            client.SetEventHandle(event.0).map_err(error)?;
        }
        let buffer_ms =
            client.GetBufferSize().map_err(error)? as f64 * 1000.0 / decoded.sample_rate as f64;
        let capture = client.GetService::<IAudioCaptureClient>().map_err(error)?;
        Ok((client, capture, event, decoded, raw_mode, buffer_ms))
    }
}
unsafe fn decode_format(pointer: *const WAVEFORMATEX) -> io::Result<CaptureFormat> {
    if pointer.is_null() {
        return Err(invalid("Missing Windows audio format."));
    }
    let format = read_unaligned(pointer);
    let mut kind = format.wFormatTag;
    let mut valid_bits = format.wBitsPerSample;
    if kind == 65534 {
        if format.cbSize < 22 {
            return Err(invalid("Truncated extended Windows format."));
        }
        let extended = read_unaligned(pointer.cast::<WAVEFORMATEXTENSIBLE>());
        valid_bits = extended.Samples.wValidBitsPerSample;
        let subtype = extended.SubFormat;
        kind = if subtype == GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71) {
            1
        } else if subtype == GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71) {
            3
        } else {
            return Err(invalid("Unsupported Windows sample encoding."));
        };
    }
    // Valid integer bits in an extended container are left aligned. Reading its full signed
    // container therefore produces the correct normalized value for 24-in-32 as well.
    let encoding = match (kind, format.wBitsPerSample) {
        (1, 16) if valid_bits <= 16 => Encoding::Signed16,
        (1, 24) if valid_bits <= 24 => Encoding::Signed24,
        (1, 32) if valid_bits <= 32 => Encoding::Signed32,
        (3, 32) => Encoding::Float32,
        _ => return Err(invalid("Unsupported Windows sample encoding.")),
    };
    let decoded = CaptureFormat {
        sample_rate: format.nSamplesPerSec,
        channels: format.nChannels,
        encoding,
        frame_stride: format.nBlockAlign as usize,
    };
    decoded.validate()?;
    Ok(decoded)
}
impl CaptureBackend for Wasapi {
    fn details(&self) -> CaptureDetails {
        self.details.clone()
    }
    fn start(&mut self) -> io::Result<()> {
        unsafe {
            self.client.Start().map_err(error)?;
        }
        self.started = true;
        Ok(())
    }
    fn wait(&mut self) -> io::Result<()> {
        if let Some(event) = &self.event {
            if unsafe { WaitForSingleObject(event.0, 5) } == WAIT_FAILED {
                return Err(io::Error::last_os_error());
            }
        } else {
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(())
    }
    fn poll(
        &mut self,
        sink: &mut dyn FnMut(CapturePacket<'_>) -> io::Result<()>,
    ) -> io::Result<()> {
        unsafe {
            for _ in 0..64 {
                if self.capture.GetNextPacketSize().map_err(error)? == 0 {
                    break;
                }
                let (mut data, mut frames, mut flags, mut position, mut qpc) =
                    (std::ptr::null_mut(), 0, 0, 0, 0);
                self.capture
                    .GetBuffer(
                        &mut data,
                        &mut frames,
                        &mut flags,
                        Some(&mut position),
                        Some(&mut qpc),
                    )
                    .map_err(error)?;
                if frames == 0 {
                    break;
                }
                let silent = flags & AUDCLNT_BUFFERFLAGS_SILENT.0 as u32 != 0;
                let length = (frames as usize).checked_mul(self.details.format.frame_stride);
                let result = if let Some(length) = length.filter(|_| {
                    frames <= self.details.format.sample_rate && (!data.is_null() || silent)
                }) {
                    let valid_time = flags & AUDCLNT_BUFFERFLAGS_TIMESTAMP_ERROR.0 as u32 == 0;
                    let bytes = if silent {
                        &[][..]
                    } else {
                        std::slice::from_raw_parts(data, length)
                    };
                    sink(CapturePacket {
                        bytes,
                        frames: frames as usize,
                        device_position: valid_time.then_some(position),
                        capture_ns: if valid_time {
                            qpc.checked_mul(100)
                                .and_then(|time| i64::try_from(time).ok())
                        } else {
                            None
                        },
                        discontinuity: flags & AUDCLNT_BUFFERFLAGS_DATA_DISCONTINUITY.0 as u32 != 0,
                        silent,
                    })
                } else {
                    Err(invalid("Invalid Windows capture packet."))
                };
                let released = self.capture.ReleaseBuffer(frames).map_err(error);
                result?;
                released?;
            }
        }
        Ok(())
    }
    fn stop(&mut self) {
        if self.started {
            unsafe {
                let _ = self.client.Stop();
            }
            self.started = false;
        }
    }
}
impl Drop for Wasapi {
    fn drop(&mut self) {
        self.stop();
    }
}
#[derive(Debug)]
struct Failure(&'static str);
impl std::fmt::Display for Failure {
    fn fmt(&self, out: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        out.write_str(self.0)
    }
}
impl std::error::Error for Failure {}
fn error(error: windows::core::Error) -> io::Error {
    let code = match error.code() {
        AUDCLNT_E_DEVICE_INVALIDATED | AUDCLNT_E_RESOURCES_INVALIDATED => "device_invalidated",
        AUDCLNT_E_DEVICE_IN_USE => "mic_silenced",
        _ => "pc_audio_error",
    };
    io::Error::other(Failure(code))
}
pub(super) fn failure_code(error: &io::Error) -> &'static str {
    error
        .get_ref()
        .and_then(|error| error.downcast_ref::<Failure>())
        .map(|error| error.0)
        .unwrap_or("pc_audio_error")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wave_format_extensible_accepts_left_aligned_pcm_and_rejects_unknown_subformats() {
        let mut format = WAVEFORMATEXTENSIBLE {
            Format: WAVEFORMATEX {
                wFormatTag: 65534,
                nChannels: 2,
                nSamplesPerSec: 48000,
                nBlockAlign: 8,
                wBitsPerSample: 32,
                cbSize: 22,
                ..Default::default()
            },
            Samples: WAVEFORMATEXTENSIBLE_0 {
                wValidBitsPerSample: 24,
            },
            SubFormat: GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71),
            ..Default::default()
        };
        let decoded = unsafe { decode_format(std::ptr::addr_of!(format.Format)) }.unwrap();
        assert!(matches!(decoded.encoding, Encoding::Signed32));
        format.SubFormat = GUID::zeroed();
        assert!(unsafe { decode_format(std::ptr::addr_of!(format.Format)) }.is_err());
        format.Format.cbSize = 0;
        assert!(unsafe { decode_format(std::ptr::addr_of!(format.Format)) }.is_err());
    }
}
