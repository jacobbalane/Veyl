//! https://learn.microsoft.com/windows/win32/coreaudio/rendering-a-stream
use super::*;
use crate::pc_source::native::{error, Apartment, Priority};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Media::Audio::*,
        System::{
            Com::{CoCreateInstance, CLSCTX_ALL},
            Threading::AvSetMmThreadCharacteristicsW,
        },
    },
};

struct Render {
    client: IAudioClient,
    render: IAudioRenderClient,
    _priority: Option<Priority>,
    _apartment: Apartment,
}
impl Drop for Render {
    fn drop(&mut self) {
        unsafe {
            let _ = self.client.Stop();
        }
    }
}
pub(super) fn play(
    device: AudioDevice,
    stop: Arc<AtomicBool>,
    started: oneshot::Sender<()>,
) -> io::Result<()> {
    let apartment = Apartment::enter()?;
    let enumerator: IMMDeviceEnumerator =
        unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL).map_err(error)? };
    let endpoint = unsafe {
        enumerator
            .GetDevice(&HSTRING::from(device.id))
            .map_err(error)?
    };
    let client: IAudioClient = unsafe { endpoint.Activate(CLSCTX_ALL, None).map_err(error)? };
    // Windows converts the generated stereo float stream into the endpoint's shared mix format.
    let format = WAVEFORMATEX {
        wFormatTag: 3,
        nChannels: 2,
        nSamplesPerSec: 48000,
        nAvgBytesPerSec: 384000,
        nBlockAlign: 8,
        wBitsPerSample: 32,
        cbSize: 0,
    };
    unsafe {
        client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_AUTOCONVERTPCM | AUDCLNT_STREAMFLAGS_SRC_DEFAULT_QUALITY,
                200_000,
                0,
                &format,
                None,
            )
            .map_err(error)?;
    }
    let render = unsafe { client.GetService::<IAudioRenderClient>().map_err(error)? };
    let capacity = unsafe { client.GetBufferSize().map_err(error)? };
    drop(endpoint);
    drop(enumerator);
    let mut task = 0;
    let priority = unsafe {
        AvSetMmThreadCharacteristicsW(w!("Pro Audio"), &mut task)
            .ok()
            .map(Priority)
    };
    let output = Render {
        client,
        render,
        _priority: priority,
        _apartment: apartment,
    };
    let sound = veyl_dsp::testsound::BrightLoop::new(48000, veyl_dsp::testsound::DEFAULT_SEED)
        .map_err(|_| io::Error::other("pc_audio_error"))?;
    let total = 48000 * 5;
    let mut cursor = 0;
    let mut running = false;
    let mut started = Some(started);
    let deadline = Instant::now() + Duration::from_secs(8);
    while cursor < total {
        if stop.load(Ordering::Acquire) {
            return Ok(());
        }
        if Instant::now() > deadline {
            return Err(io::Error::other("pc_audio_error"));
        }
        let padding = unsafe { output.client.GetCurrentPadding().map_err(error)? };
        let frames = capacity
            .saturating_sub(padding)
            .min((total - cursor) as u32);
        if frames > 0 {
            unsafe {
                let buffer = output.render.GetBuffer(frames).map_err(error)?;
                if buffer.is_null() {
                    let _ = output.render.ReleaseBuffer(0, 0);
                    return Err(io::Error::other("pc_audio_error"));
                }
                let buffer =
                    std::slice::from_raw_parts_mut(buffer.cast::<[f32; 2]>(), frames as usize);
                buffer.copy_from_slice(&sound.frames()[cursor..cursor + frames as usize]);
                output.render.ReleaseBuffer(frames, 0).map_err(error)?;
            }
            cursor += frames as usize;
        }
        if !running {
            unsafe {
                output.client.Start().map_err(error)?;
            }
            running = true;
            if let Some(started) = started.take() {
                let _ = started.send(());
            }
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    while !stop.load(Ordering::Acquire)
        && unsafe { output.client.GetCurrentPadding().map_err(error)? } > 0
    {
        if Instant::now() > deadline {
            return Err(io::Error::other("pc_audio_error"));
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
