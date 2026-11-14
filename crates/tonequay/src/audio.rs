//! CPAL owns the sole audio consumer. The callback only pops, converts and counts.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
#[derive(Default)]
pub struct Stats {
    pub missing: AtomicU64,
    pub failed: AtomicBool,
    pub started: AtomicBool,
    pub max_block: AtomicU64,
}
pub fn devices() -> Vec<String> {
    cpal::default_host()
        .output_devices()
        .map(|ds| ds.filter_map(|d| d.name().ok()).collect())
        .unwrap_or_default()
}
pub fn start(
    name: &str,
    consumer: rtrb::Consumer<[f32; 2]>,
    stats: Arc<Stats>,
    target_ms: u32,
) -> Result<(cpal::Stream, u32), String> {
    let host = cpal::default_host();
    let device = if name.is_empty() {
        host.default_output_device()
    } else {
        host.output_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.name().is_ok_and(|n| n == name))
    }
    .ok_or("audio output unavailable")?;
    let cfg = device.default_output_config().map_err(|e| e.to_string())?;
    let mut config: cpal::StreamConfig = cfg.clone().into();
    if let cpal::SupportedBufferSize::Range { min, max } = cfg.buffer_size() {
        let requested = if target_ms <= 20 {
            512
        } else if target_ms <= 50 {
            1024
        } else {
            2048
        };
        if min <= max {
            config.buffer_size = cpal::BufferSize::Fixed(requested.clamp(*min, *max));
        }
    }
    let rate = config.sample_rate.0;
    let stream = match cfg.sample_format() {
        cpal::SampleFormat::I8 => build::<i8>(&device, &config, consumer, stats),
        cpal::SampleFormat::U8 => build::<u8>(&device, &config, consumer, stats),
        cpal::SampleFormat::U32 => build::<u32>(&device, &config, consumer, stats),
        cpal::SampleFormat::I64 => build::<i64>(&device, &config, consumer, stats),
        cpal::SampleFormat::U64 => build::<u64>(&device, &config, consumer, stats),
        cpal::SampleFormat::F32 => build::<f32>(&device, &config, consumer, stats),
        cpal::SampleFormat::I16 => build::<i16>(&device, &config, consumer, stats),
        cpal::SampleFormat::U16 => build::<u16>(&device, &config, consumer, stats),
        cpal::SampleFormat::I32 => build::<i32>(&device, &config, consumer, stats),
        cpal::SampleFormat::F64 => build::<f64>(&device, &config, consumer, stats),
        other => return Err(format!("unsupported output sample format: {other:?}")),
    }?;
    stream.play().map_err(|e| e.to_string())?;
    Ok((stream, rate))
}
fn build<T: cpal::SizedSample + cpal::FromSample<f32>>(
    device: &cpal::Device,
    cfg: &cpal::StreamConfig,
    mut consumer: rtrb::Consumer<[f32; 2]>,
    stats: Arc<Stats>,
) -> Result<cpal::Stream, String> {
    let channels = usize::from(cfg.channels);
    let errors = stats.clone();
    device
        .build_output_stream(
            cfg,
            move |out: &mut [T], _| {
                stats
                    .max_block
                    .fetch_max((out.len() / channels) as u64, Ordering::Relaxed);
                for frame in out.chunks_mut(channels) {
                    let sample = if !stats.started.load(Ordering::Relaxed) {
                        [0.0; 2]
                    } else {
                        consumer.pop().unwrap_or_else(|_| {
                            if stats.started.load(Ordering::Relaxed) {
                                stats.missing.fetch_add(1, Ordering::Relaxed);
                            }
                            [0.0; 2]
                        })
                    };
                    for (c, value) in frame.iter_mut().enumerate() {
                        let v = if channels == 1 {
                            (sample[0] + sample[1]) * 0.5
                        } else if c < 2 {
                            sample[c]
                        } else {
                            0.0
                        };
                        *value = T::from_sample(v);
                    }
                }
            },
            move |_| {
                errors.failed.store(true, Ordering::Relaxed);
            },
            None,
        )
        .map_err(|e| e.to_string())
}
