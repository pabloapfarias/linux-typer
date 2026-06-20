use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{ChannelCount, SampleFormat, Stream, StreamConfig, SupportedStreamConfig};
use cpal::{FromSample, Sample};
use log::{info, warn};

use crate::error::{AppError, AppResult};

const TARGET_SAMPLE_RATE: u32 = 16_000;
const TARGET_CHANNELS: u16 = 1;

#[derive(Debug)]
struct SharedBuffer {
    samples: Vec<f32>,
    channels: usize,
}

pub struct Recorder {
    device_name: String,
    stream: Option<Stream>,
    buffer: Arc<Mutex<SharedBuffer>>,
    input_sample_rate: u32,
}

impl Recorder {
    pub fn start() -> AppResult<Self> {
        let host = cpal::default_host();
        let device = host.default_input_device().ok_or_else(|| {
            AppError::Unsupported("no default microphone/input device available".into())
        })?;
        let device_name = "default-input-device".to_string();

        let selected = select_input_config(&device)?;
        let sample_format = selected.sample_format();
        let config: StreamConfig = selected.into();
        let input_sample_rate = config.sample_rate;
        let input_channels = config.channels as usize;

        info!(
            "audio input device: {} | sample_rate={} | channels={} | sample_format={:?}",
            device_name, input_sample_rate, input_channels, sample_format
        );

        let buffer = Arc::new(Mutex::new(SharedBuffer {
            samples: Vec::new(),
            channels: input_channels,
        }));

        let err_fn = |err| {
            warn!("audio stream error: {err}");
        };

        let stream = build_input_stream(&device, &config, sample_format, buffer.clone(), err_fn)?;
        stream.play()?;

        Ok(Self {
            device_name,
            stream: Some(stream),
            buffer,
            input_sample_rate,
        })
    }

    pub fn stop_and_save(mut self, temp_dir: &Path) -> AppResult<PathBuf> {
        if let Some(stream) = self.stream.take() {
            let _ = stream.pause();
            drop(stream);
        }

        let wav_path = unique_wav_path(temp_dir)?;
        let raw = self
            .buffer
            .lock()
            .map_err(|_| AppError::Unsupported("audio buffer mutex poisoned".into()))?;

        if raw.samples.is_empty() {
            return Err(AppError::Unsupported(
                "no audio samples captured; check microphone permissions".into(),
            ));
        }

        let mono = mix_to_mono(&raw.samples, raw.channels);
        let resampled = if self.input_sample_rate == TARGET_SAMPLE_RATE {
            mono
        } else {
            linear_resample(&mono, self.input_sample_rate, TARGET_SAMPLE_RATE)
        };

        write_wav(&wav_path, &resampled)?;
        info!(
            "saved wav: {} | input_device={} | frames={}",
            wav_path.display(),
            self.device_name,
            resampled.len()
        );
        Ok(wav_path)
    }
}

fn select_input_config(device: &cpal::Device) -> AppResult<SupportedStreamConfig> {
    let configs = device.supported_input_configs()?;

    for config in configs {
        if config.channels() == TARGET_CHANNELS
            && config.min_sample_rate() <= TARGET_SAMPLE_RATE
            && config.max_sample_rate() >= TARGET_SAMPLE_RATE
        {
            return Ok(config.with_sample_rate(TARGET_SAMPLE_RATE));
        }
    }

    let default = device.default_input_config()?;
    Ok(default)
}

fn build_input_stream<E>(
    device: &cpal::Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    buffer: Arc<Mutex<SharedBuffer>>,
    err_fn: E,
) -> AppResult<Stream>
where
    E: FnMut(cpal::Error) + Send + 'static,
{
    match sample_format {
        SampleFormat::F32 => build_stream::<f32, E>(device, config, buffer, err_fn),
        SampleFormat::I16 => build_stream::<i16, E>(device, config, buffer, err_fn),
        SampleFormat::I32 => build_stream::<i32, E>(device, config, buffer, err_fn),
        SampleFormat::U8 => build_stream::<u8, E>(device, config, buffer, err_fn),
        SampleFormat::U16 => build_stream::<u16, E>(device, config, buffer, err_fn),
        SampleFormat::U32 => build_stream::<u32, E>(device, config, buffer, err_fn),
        other => Err(AppError::Unsupported(format!(
            "unsupported input sample format: {other:?}"
        ))),
    }
}

fn build_stream<T, E>(
    device: &cpal::Device,
    config: &StreamConfig,
    buffer: Arc<Mutex<SharedBuffer>>,
    mut err_fn: E,
) -> AppResult<Stream>
where
    T: Sample + cpal::SizedSample,
    f32: FromSample<T>,
    E: FnMut(cpal::Error) + Send + 'static,
{
    let channels = config.channels as usize;
    let stream = device.build_input_stream(
        config.clone(),
        move |data: &[T], _| {
            if let Ok(mut shared) = buffer.lock() {
                shared.channels = channels;
                shared
                    .samples
                    .extend(data.iter().map(|sample| f32::from_sample(*sample)));
            }
        },
        move |err| err_fn(err),
        None,
    )?;
    Ok(stream)
}

fn mix_to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
    if channels <= 1 {
        return samples.to_vec();
    }

    samples
        .chunks(channels)
        .map(|frame| frame.iter().copied().sum::<f32>() / channels as f32)
        .collect()
}

fn linear_resample(samples: &[f32], from_rate: u32, to_rate: u32) -> Vec<f32> {
    if samples.is_empty() || from_rate == to_rate {
        return samples.to_vec();
    }

    let ratio = to_rate as f64 / from_rate as f64;
    let out_len = ((samples.len() as f64) * ratio).round() as usize;
    let mut out = Vec::with_capacity(out_len);

    for i in 0..out_len {
        let position = (i as f64) / ratio;
        let left = position.floor() as usize;
        let right = (left + 1).min(samples.len().saturating_sub(1));
        let frac = (position - left as f64) as f32;
        let a = samples[left];
        let b = samples[right];
        out.push(a + (b - a) * frac);
    }

    out
}

fn write_wav(path: &Path, samples: &[f32]) -> AppResult<()> {
    let spec = hound::WavSpec {
        channels: TARGET_CHANNELS as ChannelCount,
        sample_rate: TARGET_SAMPLE_RATE,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
    };

    let mut writer = hound::WavWriter::create(path, spec)?;
    for sample in samples {
        let clamped = sample.clamp(-1.0, 1.0);
        let value = (clamped * i16::MAX as f32) as i16;
        writer.write_sample(value)?;
    }
    writer.finalize()?;
    Ok(())
}

fn unique_wav_path(temp_dir: &Path) -> AppResult<PathBuf> {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| AppError::Unsupported(format!("system clock error: {err}")))?
        .as_millis();
    Ok(temp_dir.join(format!("recording-{millis}.wav")))
}
