use super::Run;
use crate::recording::Input;
use serde::Serialize;
use std::{
    fs,
    io::{self, BufWriter, Write},
    path::Path,
};

const FILES: &[&str] = &[
    "summary.json",
    "features.csv",
    "meter.csv",
    "pokes.csv",
    "motion.png",
];
fn tag<T: Serialize>(value: &T) -> io::Result<String> {
    serde_json::to_string(value).map_err(io::Error::other)
}
fn number(value: Option<f64>) -> String {
    value.map(|value| value.to_string()).unwrap_or_default()
}
pub fn write_outputs(input: &Input, run: &Run, output: &Path) -> io::Result<()> {
    if !run.trace_collected {
        return Err(io::Error::other(
            "Full analysis export requires collected traces.",
        ));
    }
    for name in FILES {
        match fs::symlink_metadata(output.join(name)) {
            Ok(_) => {
                return Err(io::Error::other(
                    "An analysis report already exists. Choose a new --out folder.",
                ))
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    fs::create_dir_all(output)?;
    let create = |name: &str| {
        fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(output.join(name))
            .map(BufWriter::new)
    };
    let mut features = create("features.csv")?;
    writeln!(features, "frame,state,valid,settling,z_max,r_peak_cm,r_f_cm,mean_z,present,peakiness,peak_floor,peak_energy,suspended,drift_ppm,direct_residual")?;
    for row in &run.features {
        let f = row.feature;
        writeln!(
            features,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            f.frame,
            tag(&row.state)?,
            f.valid,
            f.settling,
            f.z_max,
            number(f.r_peak_cm.map(f64::from)),
            number(f.distance_cm.map(f64::from)),
            f.mean_energy,
            f.present,
            f.peakiness,
            f.peak_floor,
            f.peak_energy,
            row.suspended,
            row.drift_ppm,
            number(row.direct_residual)
        )?;
    }
    features.flush()?;
    let mut meter = create("meter.csv")?;
    writeln!(meter, "frame,score,label,tip,high,detail,volume,steady,clarity,brightness_db,bandwidth_hz,mic_band_dbfs,clarity_db,steady_fraction,mic_clip,detail_cm")?;
    for r in &run.meters {
        writeln!(
            meter,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            r.frame,
            r.score,
            tag(&r.label)?,
            tag(&r.tip)?,
            r.factors.high,
            r.factors.detail,
            r.factors.volume,
            r.factors.steady,
            r.factors.clarity,
            number(r.brightness_db.map(f64::from)),
            r.bandwidth_hz,
            r.mic_band_dbfs,
            r.clarity_db,
            r.steady_fraction,
            r.mic_clip,
            number(r.detail_cm.map(f64::from))
        )?;
    }
    meter.flush()?;
    let mut pokes = create("pokes.csv")?;
    writeln!(
        pokes,
        "turn_frame,detect_frame,distance_cm,depth_cm,confidence,confirmation_ms"
    )?;
    for p in &run.pokes {
        writeln!(
            pokes,
            "{},{},{},{},{},{}",
            p.turn_frame,
            p.detect_frame,
            p.distance_cm,
            p.depth_cm,
            p.confidence,
            (p.detect_frame as i128 - p.turn_frame as i128) as f64 * 1000.0
                / input.meta.sample_rate as f64
        )?;
    }
    pokes.flush()?;
    let picture = motion_image(input, run)?;
    let mut image = create("motion.png")?;
    image.write_all(&picture)?;
    image.flush()?;
    let mut summary = create("summary.json")?;
    serde_json::to_writer_pretty(&mut summary, &run.report).map_err(io::Error::other)?;
    writeln!(summary)?;
    summary.flush()?;
    Ok(())
}

struct Canvas {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}
impl Canvas {
    fn new(width: usize, height: usize) -> Self {
        let mut pixels = vec![0; width * height * 3];
        for pixel in pixels.as_chunks_mut::<3>().0 {
            pixel.copy_from_slice(&[13, 19, 27]);
        }
        Self {
            width,
            height,
            pixels,
        }
    }
    fn pixel(&mut self, x: usize, y: usize, color: [u8; 3]) {
        if x < self.width && y < self.height {
            let at = (y * self.width + x) * 3;
            self.pixels[at..at + 3].copy_from_slice(&color);
        }
    }
    fn line(&mut self, x: usize, top: usize, bottom: usize, color: [u8; 3], dashed: bool) {
        for y in top..bottom {
            if !dashed || y % 7 < 3 {
                self.pixel(x, y, color);
            }
        }
    }
    fn text(&mut self, mut x: usize, y: usize, text: &str, color: [u8; 3]) {
        for character in text.chars() {
            let glyph = glyph(character.to_ascii_uppercase());
            for (row, bits) in glyph.iter().enumerate() {
                for col in 0..5 {
                    if bits & (1 << (4 - col)) != 0 {
                        for dy in 0..2 {
                            for dx in 0..2 {
                                self.pixel(x + col * 2 + dx, y + row * 2 + dy, color);
                            }
                        }
                    }
                }
            }
            x += 12;
        }
    }
}
fn glyph(c: char) -> [u8; 7] {
    match c {
        '0' => [14, 17, 19, 21, 25, 17, 14],
        '1' => [4, 12, 4, 4, 4, 4, 14],
        '2' => [14, 17, 1, 2, 4, 8, 31],
        '3' => [30, 1, 1, 14, 1, 1, 30],
        '4' => [2, 6, 10, 18, 31, 2, 2],
        '5' => [31, 16, 16, 30, 1, 1, 30],
        '6' => [6, 8, 16, 30, 17, 17, 14],
        '7' => [31, 1, 2, 4, 8, 8, 8],
        '8' => [14, 17, 17, 14, 17, 17, 14],
        '9' => [14, 17, 17, 15, 1, 2, 12],
        '.' => [0, 0, 0, 0, 0, 12, 12],
        '-' => [0, 0, 0, 31, 0, 0, 0],
        'A' => [14, 17, 17, 31, 17, 17, 17],
        'B' => [30, 17, 17, 30, 17, 17, 30],
        'C' => [14, 17, 16, 16, 16, 17, 14],
        'D' => [30, 17, 17, 17, 17, 17, 30],
        'E' => [31, 16, 16, 30, 16, 16, 31],
        'F' => [31, 16, 16, 30, 16, 16, 16],
        'G' => [14, 17, 16, 23, 17, 17, 15],
        'H' => [17, 17, 17, 31, 17, 17, 17],
        'I' => [14, 4, 4, 4, 4, 4, 14],
        'J' => [7, 2, 2, 2, 2, 18, 12],
        'K' => [17, 18, 20, 24, 20, 18, 17],
        'L' => [16, 16, 16, 16, 16, 16, 31],
        'M' => [17, 27, 21, 21, 17, 17, 17],
        'N' => [17, 25, 21, 19, 17, 17, 17],
        'O' => [14, 17, 17, 17, 17, 17, 14],
        'P' => [30, 17, 17, 30, 16, 16, 16],
        'Q' => [14, 17, 17, 17, 21, 18, 13],
        'R' => [30, 17, 17, 30, 20, 18, 17],
        'S' => [15, 16, 16, 14, 1, 1, 30],
        'T' => [31, 4, 4, 4, 4, 4, 4],
        'U' => [17, 17, 17, 17, 17, 17, 14],
        'V' => [17, 17, 17, 17, 17, 10, 4],
        'W' => [17, 17, 17, 21, 21, 21, 10],
        'X' => [17, 17, 10, 4, 10, 17, 17],
        'Y' => [17, 17, 10, 4, 4, 4, 4],
        'Z' => [31, 1, 2, 4, 8, 16, 31],
        _ => [0; 7],
    }
}
fn color(db: f32) -> [u8; 3] {
    let stops = [
        [68.0, 1.0, 84.0],
        [59.0, 82.0, 139.0],
        [33.0, 145.0, 140.0],
        [94.0, 201.0, 98.0],
        [253.0, 231.0, 37.0],
    ];
    let at = db.clamp(0.0, 30.0) / 30.0 * 4.0;
    let i = (at.floor() as usize).min(3);
    std::array::from_fn(|c| {
        (stops[i][c] + (stops[i + 1][c] - stops[i][c]) * (at - i as f32)).round() as u8
    })
}
fn motion_image(input: &Input, run: &Run) -> io::Result<Vec<u8>> {
    let seconds = run.report.streams.microphone.span_frames as f64 / input.meta.sample_rate as f64;
    let plot_width = (seconds * 40.0).ceil().clamp(640.0, 8192.0) as usize;
    let (left, top, plot_height) = (64, 40, 360);
    let mut canvas = Canvas::new(plot_width + left + 24, 500);
    let frame_to_x = |frame: u64| {
        ((frame.saturating_sub(input.meta.mic_first_frame) as f64
            / run.report.streams.microphone.span_frames.max(1) as f64
            * plot_width as f64)
            .floor() as usize)
            .min(plot_width - 1)
    };
    let mut columns = vec![[0.0_f32; 60]; plot_width];
    let mut valid = vec![false; plot_width];
    for row in &run.motion {
        let x = frame_to_x(row.frame);
        if row.valid {
            valid[x] = true;
            for (peak, value) in columns[x].iter_mut().zip(row.values) {
                if !value.is_finite() {
                    return Err(io::Error::other("The engine produced non-finite motion."));
                }
                *peak = peak.max(value);
            }
        }
    }
    for (x, (column, is_valid)) in columns.iter().zip(&valid).enumerate() {
        for (distance, value) in column.iter().enumerate() {
            let tint = if *is_valid {
                color(*value)
            } else {
                [20, 29, 39]
            };
            for dy in 0..6 {
                canvas.pixel(left + x, top + (59 - distance) * 6 + dy, tint);
            }
        }
    }
    let white = [216, 225, 232];
    canvas.text(left, 11, "VEYL MOTION", white);
    canvas.text(6, 11, "CM", white);
    for cm in (0..=60).step_by(10) {
        let y = top + plot_height - (cm * 6).min(plot_height);
        canvas.text(24, y.saturating_sub(7), &cm.to_string(), white);
        for x in left..left + plot_width {
            if x % 8 == 0 {
                canvas.pixel(x, y.min(top + plot_height - 1), [95, 107, 118]);
            }
        }
    }
    for i in 0..=5 {
        let x = left + i * (plot_width - 1) / 5;
        let label = format!("{:.1}", seconds * i as f64 / 5.0);
        canvas.text(
            x.saturating_sub(18)
                .min(canvas.width - label.len() * 12 - 4),
            top + plot_height + 11,
            &label,
            white,
        );
    }
    canvas.text(
        left + plot_width / 2 - 42,
        top + plot_height + 33,
        "SECONDS",
        white,
    );
    for label in input
        .labels
        .iter()
        .filter(|label| matches!(label.kind.as_str(), "poke_truth" | "poke_mark"))
    {
        canvas.line(
            left + frame_to_x(label.frame),
            top,
            top + plot_height,
            [225, 229, 235],
            true,
        );
    }
    for poke in &run.pokes {
        let x = left + frame_to_x(poke.turn_frame);
        canvas.line(x, top, top + plot_height, [247, 133, 85], false);
        let y = top + ((60.0 - poke.distance_cm.clamp(0.0, 60.0)) * 6.0) as usize;
        for dy in 0..5 {
            for dx in 0..5 {
                canvas.pixel(
                    (x + dx).saturating_sub(2),
                    (y + dy).saturating_sub(2),
                    [255, 238, 225],
                );
            }
        }
    }
    let y = 470;
    canvas.text(left, y, "LABELS", white);
    canvas.text(left + 100, y, "POKES", [247, 133, 85]);
    canvas.text(left + 180, y, "NO DATA", [124, 140, 155]);
    let bar = left + plot_width - 220;
    for x in 0..150 {
        for dy in 0..12 {
            canvas.pixel(bar + x, y + dy, color(x as f32 / 149.0 * 30.0));
        }
    }
    canvas.text(bar.saturating_sub(18), y, "0", white);
    canvas.text(bar + 157, y, "30 DB", white);
    let mut bytes = vec![];
    let mut encoder = png::Encoder::new(&mut bytes, canvas.width as u32, canvas.height as u32);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .add_text_chunk(
            "Title".into(),
            "Veyl motion: echo distance, microphone time, normalized motion 0-30 dB".into(),
        )
        .map_err(io::Error::other)?;
    let mut writer = encoder.write_header().map_err(io::Error::other)?;
    writer
        .write_image_data(&canvas.pixels)
        .map_err(io::Error::other)?;
    writer.finish().map_err(io::Error::other)?;
    Ok(bytes)
}
