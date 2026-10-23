//! Parameter comparisons replay the same input; labels never enter the engine.
use crate::{
    analysis::{self, PokeStatistics},
    analyze,
    recording::Input,
};
use serde_json::Value;
use std::{
    collections::BTreeSet,
    fs,
    io::{self, BufWriter, Write},
    path::{Path, PathBuf},
};
use veyl_dsp::config::EngineConfig;
#[cfg(test)]
mod tests;

pub struct Parameter {
    pub name: String,
    pub values: Vec<Value>,
}
pub struct SweepRow {
    pub values: Vec<Value>,
    pub metrics: PokeStatistics,
}
pub struct SweepReport {
    pub parameters: Vec<String>,
    pub rows: Vec<SweepRow>,
    pub best: Vec<usize>,
}
pub fn parameter(text: &str) -> io::Result<Parameter> {
    let (name, values) = text
        .split_once('=')
        .ok_or_else(|| invalid("Use --set parameter=value,value."))?;
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
        || !serde_json::to_value(EngineConfig::default())
            .map_err(io::Error::other)?
            .as_object()
            .unwrap()
            .contains_key(name)
    {
        return Err(invalid("Unknown engine parameter."));
    }
    let values = values
        .split(',')
        .map(|value| {
            let parsed: Value = serde_json::from_str(value)
                .map_err(|_| invalid("Parameter values must be finite numbers or booleans."))?;
            if !parsed.is_number() && !parsed.is_boolean() {
                return Err(invalid(
                    "Parameter values must be finite numbers or booleans.",
                ));
            }
            Ok(parsed)
        })
        .collect::<io::Result<Vec<_>>>()?;
    if values.is_empty() || values.len() > 32 {
        return Err(invalid("Use between one and 32 values per parameter."));
    }
    Ok(Parameter {
        name: name.into(),
        values,
    })
}
fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message)
}
fn combinations(parameters: &[Parameter]) -> io::Result<Vec<Vec<Value>>> {
    if parameters.is_empty() || parameters.len() > 16 {
        return Err(invalid("Use between one and 16 --set parameters."));
    }
    let mut names = BTreeSet::new();
    let mut result = vec![vec![]];
    for parameter in parameters {
        if !names.insert(&parameter.name) {
            return Err(invalid("A sweep parameter was repeated."));
        }
        let count = result
            .len()
            .checked_mul(parameter.values.len())
            .filter(|count| *count <= 4096)
            .ok_or_else(|| invalid("The sweep exceeds 4096 combinations."))?;
        let mut next = Vec::with_capacity(count);
        for row in &result {
            for value in &parameter.values {
                let mut new = row.clone();
                new.push(value.clone());
                next.push(new);
            }
        }
        result = next;
    }
    Ok(result)
}
struct Aggregate {
    stats: PokeStatistics,
    seconds: f64,
    latency: Vec<f64>,
    confirmation: Vec<f64>,
    marks: Vec<f64>,
}
impl Aggregate {
    fn new() -> Self {
        Self {
            stats: PokeStatistics {
                annotations_available: true,
                ..PokeStatistics::default()
            },
            seconds: 0.0,
            latency: vec![],
            confirmation: vec![],
            marks: vec![],
        }
    }
    fn add(&mut self, input: &Input, run: &analysis::Run) {
        let samples =
            analysis::sample_pokes(input, &run.pokes, run.report.streams.microphone.span_frames);
        let stats = &samples.statistics;
        self.stats.detected += stats.detected;
        self.stats.labels += stats.labels;
        self.stats.matched += stats.matched;
        self.stats.unmatched_detections += stats.unmatched_detections;
        self.stats.truth_labels += stats.truth_labels;
        self.stats.human_marks += stats.human_marks;
        self.stats.annotations_available &= stats.annotations_available;
        self.seconds +=
            run.report.streams.microphone.span_frames as f64 / input.meta.sample_rate as f64;
        self.latency.extend(samples.latency);
        self.confirmation.extend(samples.confirmation);
        self.marks.extend(samples.mark_offsets);
    }
    fn finish(mut self) -> PokeStatistics {
        let stats = &mut self.stats;
        stats.recall = (stats.annotations_available && stats.labels > 0)
            .then(|| stats.matched as f64 / stats.labels as f64);
        stats.precision = (stats.annotations_available && stats.detected > 0)
            .then(|| stats.matched as f64 / stats.detected as f64);
        stats.false_pokes = stats
            .annotations_available
            .then_some(stats.unmatched_detections);
        stats.false_pokes_per_minute = (stats.annotations_available && self.seconds > 0.0)
            .then(|| stats.unmatched_detections as f64 * 60.0 / self.seconds);
        stats.f1 = if stats.annotations_available {
            stats.recall.map(|r| {
                let p = stats.precision.unwrap_or(0.0);
                if r + p > 0.0 {
                    2.0 * r * p / (r + p)
                } else {
                    0.0
                }
            })
        } else {
            None
        };
        stats.latency_median_ms = analysis::percentile(&mut self.latency, 0.5);
        stats.latency_p95_ms = analysis::percentile(&mut self.latency, 0.95);
        stats.confirmation_median_ms = analysis::percentile(&mut self.confirmation, 0.5);
        stats.confirmation_p95_ms = analysis::percentile(&mut self.confirmation, 0.95);
        stats.human_mark_offset_median_ms = analysis::percentile(&mut self.marks, 0.5);
        self.stats
    }
}
pub fn run(
    directories: &[PathBuf],
    parameters: &[Parameter],
    output: &Path,
) -> io::Result<SweepReport> {
    if directories.is_empty() || directories.len() > 32 {
        return Err(invalid("Use between one and 32 input folders."));
    }
    match fs::symlink_metadata(output) {
        Ok(_) => {
            return Err(invalid(
                "The sweep output already exists. Choose a new --out path.",
            ))
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => {}
        Err(error) => return Err(error),
    }
    let combinations = combinations(parameters)?;
    let mut totals: Vec<_> = combinations.iter().map(|_| Aggregate::new()).collect();
    for directory in directories {
        let input = Input::read(directory)?;
        let stats = analyze::statistics(&input)?;
        let base =
            serde_json::to_value(analysis::default_config(&input)).map_err(io::Error::other)?;
        let configs = combinations
            .iter()
            .map(|values| {
                let mut settings = base.clone();
                for (parameter, value) in parameters.iter().zip(values) {
                    settings[&parameter.name] = value.clone();
                }
                let config: EngineConfig = serde_json::from_value(settings)
                    .map_err(|_| invalid("Invalid parameter type or value."))?;
                config.validate().map_err(invalid)?;
                Ok(config)
            })
            .collect::<io::Result<Vec<_>>>()?;
        for (index, config) in configs.into_iter().enumerate() {
            let evaluated =
                analysis::evaluate_with_statistics(&input, config, stats.clone(), false)?;
            totals[index].add(&input, &evaluated);
        }
    }
    let rows: Vec<_> = combinations
        .into_iter()
        .zip(totals)
        .map(|(values, total)| SweepRow {
            values,
            metrics: total.finish(),
        })
        .collect();
    let mut best: Vec<_> = (0..rows.len())
        .filter(|index| {
            rows[*index].metrics.f1.is_some()
                && rows[*index]
                    .metrics
                    .false_pokes_per_minute
                    .is_some_and(|value| value <= 0.2)
        })
        .collect();
    best.sort_by(|a, b| {
        rows[*b]
            .metrics
            .f1
            .unwrap()
            .total_cmp(&rows[*a].metrics.f1.unwrap())
            .then_with(|| {
                rows[*a]
                    .metrics
                    .false_pokes_per_minute
                    .unwrap()
                    .total_cmp(&rows[*b].metrics.false_pokes_per_minute.unwrap())
            })
            .then_with(|| a.cmp(b))
    });
    if let Some(parent) = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    let mut writer = BufWriter::new(
        fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(output)?,
    );
    writeln!(writer, "{},recall,precision,f1,false_pokes_per_minute,latency_median_ms,latency_p95_ms,confirmation_median_ms,labels,detected,matched,false_pokes,input_count", parameters.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(","))?;
    let number = |value: Option<f64>| value.map(|v| v.to_string()).unwrap_or_default();
    for row in &rows {
        let m = &row.metrics;
        writeln!(
            writer,
            "{},{},{},{},{},{},{},{},{},{},{},{},{}",
            row.values
                .iter()
                .map(Value::to_string)
                .collect::<Vec<_>>()
                .join(","),
            number(m.recall),
            number(m.precision),
            number(m.f1),
            number(m.false_pokes_per_minute),
            number(m.latency_median_ms),
            number(m.latency_p95_ms),
            number(m.confirmation_median_ms),
            m.labels,
            m.detected,
            m.matched,
            m.false_pokes.map(|v| v.to_string()).unwrap_or_default(),
            directories.len()
        )?;
    }
    writer.flush()?;
    Ok(SweepReport {
        parameters: parameters.iter().map(|p| p.name.clone()).collect(),
        rows,
        best,
    })
}
