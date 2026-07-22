use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{Datelike, NaiveDateTime, Timelike};
use csv::{ReaderBuilder, StringRecord, Trim};

use crate::config::{AutoChannelGroup, VoltageMode};
use crate::error::{AppError, AppResult};

pub const NUMERIC_HEADERS: [&str; 32] = [
    "UA(V)",
    "UB(V)",
    "UC(V)",
    "ULN(V)",
    "UAB(V)",
    "UBC(V)",
    "UCA(V)",
    "ULL(V)",
    "IA(A)",
    "IB(A)",
    "IC(A)",
    "I(A)",
    "PA(kW)",
    "PB(kW)",
    "PC(kW)",
    "P(kW)",
    "QA(kvar)",
    "QB(kvar)",
    "QC(kvar)",
    "Q(kvar)",
    "SA(kVA)",
    "SB(kVA)",
    "SC(kVA)",
    "S(kVA)",
    "PFA",
    "PFB",
    "PFC",
    "PF",
    "FREQ(Hz)",
    "IN(A)",
    "U_UNBL(%)",
    "I_UNBL(%)",
];

/// Primary THD columns shared by Acuvim THD CSVs and Yokogawa Auto Uthd/Ithd.
pub const THD_HEADERS: [&str; 8] = [
    "UA_THD(%)",
    "UB_THD(%)",
    "UC_THD(%)",
    "U_THD(%)",
    "IA_THD(%)",
    "IB_THD(%)",
    "IC_THD(%)",
    "I_THD(%)",
];

/// Phase-angle columns.
/// Meter: voltage phasors stay absolute; IA/IB/IC columns are converted to
/// per-phase displacement (Iφ − Uφ) so they match Yokogawa Phi-*.
/// Auto: voltage angles N/A; current columns = Phi-A/B/C.
pub const PHASE_HEADERS: [&str; 6] = [
    "UA(deg)",
    "UB(deg)",
    "UC(deg)",
    "IA_UA(deg)",
    "IB_UA(deg)",
    "IC_UA(deg)",
];

const SQRT_3: f64 = 1.732_050_807_568_877_2;
const NEAR_ZERO: f64 = 1.0e-9;

#[derive(Clone, Debug)]
pub struct MeasurementRow {
    pub timestamp: String,
    pub timestamp_epoch_seconds: i64,
    pub values: Vec<Option<f64>>,
}

impl MeasurementRow {
    pub fn value(&self, headers: &[&str], header: &str) -> Option<f64> {
        headers
            .iter()
            .position(|candidate| candidate.eq_ignore_ascii_case(header))
            .and_then(|index| self.values.get(index).copied().flatten())
    }
}

#[derive(Clone, Debug)]
pub struct MeasurementTable {
    pub source_path: PathBuf,
    pub headers: &'static [&'static str],
    pub rows: Vec<MeasurementRow>,
    pub ignored_source_columns: usize,
}

impl MeasurementTable {
    pub fn headers(&self) -> &'static [&'static str] {
        self.headers
    }

    pub fn value(&self, row: &MeasurementRow, header: &str) -> Option<f64> {
        row.value(self.headers, header)
    }
}

#[derive(Clone, Debug)]
pub struct RawAutoData {
    source_path: PathBuf,
    lookup: HashMap<String, usize>,
    rows: Vec<RawAutoRow>,
    ignored_source_columns: usize,
}

#[derive(Clone, Debug)]
struct RawAutoRow {
    source_row: usize,
    record: StringRecord,
}

pub fn preprocess_acuvim(path: impl AsRef<Path>) -> AppResult<MeasurementTable> {
    let path = path.as_ref();
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_path(path)?;
    let headers = normalize_headers(reader.headers()?);
    let lookup = header_lookup(&headers);
    let time_index = required_column(&lookup, "Time", path)?;
    let value_indices = NUMERIC_HEADERS
        .iter()
        .map(|header| required_column(&lookup, header, path))
        .collect::<AppResult<Vec<_>>>()?;

    let mut rows = Vec::new();
    for (record_index, record) in reader.records().enumerate() {
        let record = record?;
        if should_skip_record(&record) {
            continue;
        }
        let source_row = record_index + 2;
        let timestamp = record.get(time_index).unwrap_or_default().trim();
        let parsed_timestamp = parse_meter_timestamp(timestamp).ok_or_else(|| {
            AppError::Message(format!(
                "Invalid Acuvim timestamp '{}' at {} row {source_row}",
                timestamp,
                path.display()
            ))
        })?;
        let mut values = Vec::with_capacity(NUMERIC_HEADERS.len());
        for (header, index) in NUMERIC_HEADERS.iter().zip(&value_indices) {
            values.push(parse_optional_number(
                record.get(*index).unwrap_or_default(),
                path,
                source_row,
                header,
            )?);
        }
        rows.push(MeasurementRow {
            timestamp: timestamp.to_owned(),
            timestamp_epoch_seconds: parsed_timestamp.and_utc().timestamp(),
            values,
        });
    }

    validate_table(path, &NUMERIC_HEADERS, rows, 0, "I(A)")
}

pub fn preprocess_auto(
    path: impl AsRef<Path>,
    group: &AutoChannelGroup,
) -> AppResult<MeasurementTable> {
    let raw = read_auto_csv(path)?;
    preprocess_auto_data(&raw, group)
}

/// Resolve Acuvim companion CSV next to a Real-Time file (`Real-Time` → `THD` / `PhaseAngle`).
pub fn companion_csv_path(real_time_path: impl AsRef<Path>, kind: &str) -> Option<PathBuf> {
    let path = real_time_path.as_ref();
    let name = path.file_name()?.to_str()?;
    let replaced = replace_realtime_token(name, kind)?;
    let candidate = path.with_file_name(replaced);
    candidate.is_file().then_some(candidate)
}

fn replace_realtime_token(file_name: &str, kind: &str) -> Option<String> {
    let lower = file_name.to_ascii_lowercase();
    let needle = "real-time";
    let start = lower.find(needle)?;
    let end = start + needle.len();
    let mut out = String::new();
    out.push_str(&file_name[..start]);
    out.push_str(kind);
    out.push_str(&file_name[end..]);
    Some(out)
}

pub fn preprocess_acuvim_thd(path: impl AsRef<Path>) -> AppResult<MeasurementTable> {
    preprocess_acuvim_metric_csv(path, &THD_HEADERS, "UA_THD(%)")
}

pub fn preprocess_acuvim_phase(path: impl AsRef<Path>) -> AppResult<MeasurementTable> {
    let mut table = preprocess_acuvim_metric_csv(path, &PHASE_HEADERS, "UA(deg)")?;
    // Acuvim exports current angles relative to UA. Auto Phi-* is per-phase
    // displacement (I vs that phase's U). Convert meter currents to the same
    // definition: Iφ − Uφ, signed to (-180, 180].
    let ua_i = header_index(&PHASE_HEADERS, "UA(deg)");
    let ub_i = header_index(&PHASE_HEADERS, "UB(deg)");
    let uc_i = header_index(&PHASE_HEADERS, "UC(deg)");
    let ia_i = header_index(&PHASE_HEADERS, "IA_UA(deg)");
    let ib_i = header_index(&PHASE_HEADERS, "IB_UA(deg)");
    let ic_i = header_index(&PHASE_HEADERS, "IC_UA(deg)");
    for row in &mut table.rows {
        if let (Some(ua_i), Some(ub_i), Some(uc_i), Some(ia_i), Some(ib_i), Some(ic_i)) =
            (ua_i, ub_i, uc_i, ia_i, ib_i, ic_i)
        {
            let ua = row.values.get(ua_i).copied().flatten();
            let ub = row.values.get(ub_i).copied().flatten();
            let uc = row.values.get(uc_i).copied().flatten();
            let ia = row.values.get(ia_i).copied().flatten();
            let ib = row.values.get(ib_i).copied().flatten();
            let ic = row.values.get(ic_i).copied().flatten();
            // Always assign conversion result (including None) so a missing voltage
            // never leaves a raw UA-relative current angle mislabeled as displacement.
            row.values[ia_i] = displacement_degrees(ia, ua);
            row.values[ib_i] = displacement_degrees(ib, ub);
            row.values[ic_i] = displacement_degrees(ic, uc);
            // Keep voltage phasors as signed absolute angles for wiring checks.
            if let Some(Some(value)) = row.values.get_mut(ua_i) {
                *value = normalize_signed_degrees(*value);
            }
            if let Some(Some(value)) = row.values.get_mut(ub_i) {
                *value = normalize_signed_degrees(*value);
            }
            if let Some(Some(value)) = row.values.get_mut(uc_i) {
                *value = normalize_signed_degrees(*value);
            }
        }
    }
    Ok(table)
}

fn header_index(headers: &[&str], name: &str) -> Option<usize> {
    headers
        .iter()
        .position(|candidate| candidate.eq_ignore_ascii_case(name))
}

fn displacement_degrees(current_vs_ua: Option<f64>, voltage_vs_ua: Option<f64>) -> Option<f64> {
    let (Some(current), Some(voltage)) = (current_vs_ua, voltage_vs_ua) else {
        return None;
    };
    Some(normalize_signed_degrees(current - voltage))
}

fn preprocess_acuvim_metric_csv(
    path: impl AsRef<Path>,
    headers: &'static [&'static str],
    sample_header: &str,
) -> AppResult<MeasurementTable> {
    let path = path.as_ref();
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_path(path)?;
    let file_headers = normalize_headers(reader.headers()?);
    let lookup = header_lookup(&file_headers);
    let time_index = required_column(&lookup, "Time", path)?;
    let value_indices = headers
        .iter()
        .map(|header| required_column(&lookup, header, path))
        .collect::<AppResult<Vec<_>>>()?;

    let mut rows = Vec::new();
    for (record_index, record) in reader.records().enumerate() {
        let record = record?;
        if should_skip_record(&record) {
            continue;
        }
        let source_row = record_index + 2;
        let timestamp = record.get(time_index).unwrap_or_default().trim();
        let parsed_timestamp = parse_meter_timestamp(timestamp).ok_or_else(|| {
            AppError::Message(format!(
                "Invalid Acuvim timestamp '{}' at {} row {source_row}",
                timestamp,
                path.display()
            ))
        })?;
        let mut values = Vec::with_capacity(headers.len());
        for (header, index) in headers.iter().zip(&value_indices) {
            values.push(parse_optional_number(
                record.get(*index).unwrap_or_default(),
                path,
                source_row,
                header,
            )?);
        }
        rows.push(MeasurementRow {
            timestamp: timestamp.to_owned(),
            timestamp_epoch_seconds: parsed_timestamp.and_utc().timestamp(),
            values,
        });
    }

    validate_table(path, headers, rows, 0, sample_header)
}

pub fn preprocess_auto_thd(
    raw: &RawAutoData,
    group: &AutoChannelGroup,
) -> AppResult<MeasurementTable> {
    let path = &raw.source_path;
    let lookup = &raw.lookup;
    for phase in &group.phases {
        required_column(lookup, &format!("Uthd-{phase}"), path)?;
        required_column(lookup, &format!("Ithd-{phase}"), path)?;
    }

    let mut rows = Vec::new();
    for raw_row in &raw.rows {
        let record = &raw_row.record;
        let source_row = raw_row.source_row;
        let date = raw_value(record, lookup, "Date");
        let time = raw_value(record, lookup, "Time");
        let parsed_timestamp = parse_auto_timestamp(date, time).ok_or_else(|| {
            AppError::Message(format!(
                "Invalid Auto timestamp '{date} {time}' at {} row {source_row}",
                path.display()
            ))
        })?;
        let numeric = |column: &str| {
            parse_optional_number(raw_value(record, lookup, column), path, source_row, column)
        };
        let phase_a = &group.phases[0];
        let phase_b = &group.phases[1];
        let phase_c = &group.phases[2];
        let u = [
            numeric(&format!("Uthd-{phase_a}"))?,
            numeric(&format!("Uthd-{phase_b}"))?,
            numeric(&format!("Uthd-{phase_c}"))?,
        ];
        let i = [
            numeric(&format!("Ithd-{phase_a}"))?,
            numeric(&format!("Ithd-{phase_b}"))?,
            numeric(&format!("Ithd-{phase_c}"))?,
        ];
        rows.push(MeasurementRow {
            timestamp: format_meter_timestamp(parsed_timestamp),
            timestamp_epoch_seconds: parsed_timestamp.and_utc().timestamp(),
            values: vec![
                u[0],
                u[1],
                u[2],
                average_three(u),
                i[0],
                i[1],
                i[2],
                average_three(i),
            ],
        });
    }
    validate_table(
        path,
        &THD_HEADERS,
        rows,
        raw.ignored_source_columns,
        "U_THD(%)",
    )
}

pub fn preprocess_auto_phase(
    raw: &RawAutoData,
    group: &AutoChannelGroup,
) -> AppResult<MeasurementTable> {
    let path = &raw.source_path;
    let lookup = &raw.lookup;
    for phase in &group.phases {
        required_column(lookup, &format!("Phi-{phase}"), path)?;
    }

    let mut rows = Vec::new();
    for raw_row in &raw.rows {
        let record = &raw_row.record;
        let source_row = raw_row.source_row;
        let date = raw_value(record, lookup, "Date");
        let time = raw_value(record, lookup, "Time");
        let parsed_timestamp = parse_auto_timestamp(date, time).ok_or_else(|| {
            AppError::Message(format!(
                "Invalid Auto timestamp '{date} {time}' at {} row {source_row}",
                path.display()
            ))
        })?;
        let numeric = |column: &str| {
            parse_optional_number(raw_value(record, lookup, column), path, source_row, column)
        };
        let phase_a = &group.phases[0];
        let phase_b = &group.phases[1];
        let phase_c = &group.phases[2];
        // Auto Phi is displacement angle; voltage absolute angles are not in Auto.
        // Yokogawa lagging PF often reports +Phi while Acuvim I−U displacement is
        // negative for the same lagging load — store −Phi so Δdeg is near zero when
        // magnitudes match.
        rows.push(MeasurementRow {
            timestamp: format_meter_timestamp(parsed_timestamp),
            timestamp_epoch_seconds: parsed_timestamp.and_utc().timestamp(),
            values: vec![
                None,
                None,
                None,
                numeric(&format!("Phi-{phase_a}"))?.map(|value| normalize_signed_degrees(-value)),
                numeric(&format!("Phi-{phase_b}"))?.map(|value| normalize_signed_degrees(-value)),
                numeric(&format!("Phi-{phase_c}"))?.map(|value| normalize_signed_degrees(-value)),
            ],
        });
    }
    validate_table(
        path,
        &PHASE_HEADERS,
        rows,
        raw.ignored_source_columns,
        "IA_UA(deg)",
    )
}

pub fn read_auto_csv(path: impl AsRef<Path>) -> AppResult<RawAutoData> {
    let path = path.as_ref();
    let mut reader = ReaderBuilder::new()
        .flexible(true)
        .trim(Trim::All)
        .from_path(path)?;
    let headers = normalize_headers(reader.headers()?);
    let ignored_source_columns = headers
        .iter()
        .filter(|header| is_junk_header(header))
        .count();
    let lookup = header_lookup(&headers);
    required_column(&lookup, "Date", path)?;
    required_column(&lookup, "Time", path)?;
    let mut rows = Vec::new();
    for (record_index, record) in reader.records().enumerate() {
        let record = record?;
        if should_skip_record(&record) {
            continue;
        }
        rows.push(RawAutoRow {
            source_row: record_index + 2,
            record,
        });
    }
    if rows.is_empty() {
        return Err(AppError::Message(format!(
            "No usable data rows were found in {}",
            path.display()
        )));
    }
    Ok(RawAutoData {
        source_path: path.to_path_buf(),
        lookup,
        rows,
        ignored_source_columns,
    })
}

pub fn preprocess_auto_data(
    raw: &RawAutoData,
    group: &AutoChannelGroup,
) -> AppResult<MeasurementTable> {
    let path = &raw.source_path;
    let lookup = &raw.lookup;
    let required = required_auto_columns(group);
    for column in &required {
        required_column(&lookup, column, path)?;
    }

    let mut rows = Vec::new();
    for raw_row in &raw.rows {
        let record = &raw_row.record;
        let source_row = raw_row.source_row;
        let date = raw_value(&record, &lookup, "Date");
        let time = raw_value(&record, &lookup, "Time");
        let parsed_timestamp = parse_auto_timestamp(date, time).ok_or_else(|| {
            AppError::Message(format!(
                "Invalid Auto timestamp '{} {}' at {} row {source_row}",
                date,
                time,
                path.display()
            ))
        })?;

        let numeric = |column: &str| {
            parse_optional_number(
                raw_value(&record, &lookup, column),
                path,
                source_row,
                column,
            )
        };
        let phase_a = &group.phases[0];
        let phase_b = &group.phases[1];
        let phase_c = &group.phases[2];

        let source_voltages = [
            numeric(&format!("Uac-{phase_a}"))?,
            numeric(&format!("Uac-{phase_b}"))?,
            numeric(&format!("Uac-{phase_c}"))?,
        ];
        let (ua, ub, uc, uln, uab, ubc, uca, ull) = match group.voltage_mode {
            VoltageMode::LineToNeutral => {
                let average = average_three(source_voltages);
                (
                    source_voltages[0],
                    source_voltages[1],
                    source_voltages[2],
                    average,
                    source_voltages[0].map(|value| value * SQRT_3),
                    source_voltages[1].map(|value| value * SQRT_3),
                    source_voltages[2].map(|value| value * SQRT_3),
                    average.map(|value| value * SQRT_3),
                )
            }
            VoltageMode::LineToLine => (
                None,
                None,
                None,
                None,
                source_voltages[0],
                source_voltages[1],
                source_voltages[2],
                average_three(source_voltages),
            ),
        };

        let currents = [
            numeric(&format!("Iac-{phase_a}"))?,
            numeric(&format!("Iac-{phase_b}"))?,
            numeric(&format!("Iac-{phase_c}"))?,
        ];
        let current_total =
            numeric(&format!("Iac-{}", group.total))?.or_else(|| average_three(currents));

        let real_power = [
            scale(numeric(&format!("P-{phase_a}"))?, 1.0 / 1000.0),
            scale(numeric(&format!("P-{phase_b}"))?, 1.0 / 1000.0),
            scale(numeric(&format!("P-{phase_c}"))?, 1.0 / 1000.0),
        ];
        let real_total = scale(numeric(&format!("P-{}", group.total))?, 1.0 / 1000.0)
            .or_else(|| sum_three(real_power));

        let apparent_power = [
            scale(numeric(&format!("S-{phase_a}"))?, 1.0 / 1000.0),
            scale(numeric(&format!("S-{phase_b}"))?, 1.0 / 1000.0),
            scale(numeric(&format!("S-{phase_c}"))?, 1.0 / 1000.0),
        ];
        let apparent_total = scale(numeric(&format!("S-{}", group.total))?, 1.0 / 1000.0)
            .or_else(|| sum_three(apparent_power));

        // Prefer Yokogawa Q-* (var → kvar). Fall back to triangle |Q| only if blank/NAN.
        let reactive_values = [
            scale(numeric(&format!("Q-{phase_a}"))?, 1.0 / 1000.0)
                .or_else(|| reactive_power(apparent_power[0], real_power[0])),
            scale(numeric(&format!("Q-{phase_b}"))?, 1.0 / 1000.0)
                .or_else(|| reactive_power(apparent_power[1], real_power[1])),
            scale(numeric(&format!("Q-{phase_c}"))?, 1.0 / 1000.0)
                .or_else(|| reactive_power(apparent_power[2], real_power[2])),
        ];
        let reactive_total = scale(numeric(&format!("Q-{}", group.total))?, 1.0 / 1000.0)
            .or_else(|| sum_three(reactive_values))
            .or_else(|| reactive_power(apparent_total, real_total));
        let power_factors = [
            numeric(&format!("PF-{phase_a}"))?,
            numeric(&format!("PF-{phase_b}"))?,
            numeric(&format!("PF-{phase_c}"))?,
        ];
        let power_factor_total =
            numeric(&format!("PF-{}", group.total))?.or_else(|| ratio(real_total, apparent_total));
        let frequency = numeric(&format!("FreqU-{phase_a}"))?;
        let voltage_unbalance = unbalance(source_voltages);
        let current_unbalance = unbalance(currents);
        let neutral_current = neutral_current(currents);

        rows.push(MeasurementRow {
            timestamp: format_meter_timestamp(parsed_timestamp),
            timestamp_epoch_seconds: parsed_timestamp.and_utc().timestamp(),
            values: vec![
                ua,
                ub,
                uc,
                uln,
                uab,
                ubc,
                uca,
                ull,
                currents[0],
                currents[1],
                currents[2],
                current_total,
                real_power[0],
                real_power[1],
                real_power[2],
                real_total,
                reactive_values[0],
                reactive_values[1],
                reactive_values[2],
                reactive_total,
                apparent_power[0],
                apparent_power[1],
                apparent_power[2],
                apparent_total,
                power_factors[0],
                power_factors[1],
                power_factors[2],
                power_factor_total,
                frequency,
                neutral_current,
                voltage_unbalance,
                current_unbalance,
            ],
        });
    }

    validate_table(
        path,
        &NUMERIC_HEADERS,
        rows,
        raw.ignored_source_columns,
        "I(A)",
    )
}

fn validate_table(
    path: &Path,
    headers: &'static [&'static str],
    rows: Vec<MeasurementRow>,
    ignored_source_columns: usize,
    sample_header: &str,
) -> AppResult<MeasurementTable> {
    if rows.is_empty() {
        return Err(AppError::Message(format!(
            "No usable data rows were found in {}",
            path.display()
        )));
    }
    if !rows
        .iter()
        .any(|row| row.value(headers, sample_header).is_some())
    {
        return Err(AppError::Message(format!(
            "No numeric {sample_header} values were found in {}",
            path.display()
        )));
    }
    Ok(MeasurementTable {
        source_path: path.to_path_buf(),
        headers,
        rows,
        ignored_source_columns,
    })
}

/// Map angle into (-180, 180] degrees.
pub fn normalize_signed_degrees(value: f64) -> f64 {
    let mut wrapped = value % 360.0;
    if wrapped > 180.0 {
        wrapped -= 360.0;
    } else if wrapped <= -180.0 {
        wrapped += 360.0;
    }
    wrapped
}

/// Smallest signed difference meter − auto in degrees, accounting for wrap.
pub fn circular_delta_degrees(meter: f64, auto: f64) -> f64 {
    let mut delta = normalize_signed_degrees(meter) - normalize_signed_degrees(auto);
    if delta > 180.0 {
        delta -= 360.0;
    } else if delta <= -180.0 {
        delta += 360.0;
    }
    delta
}

fn normalize_headers(headers: &StringRecord) -> Vec<String> {
    headers
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let header = if index == 0 {
                header.trim_start_matches('\u{feff}')
            } else {
                header
            };
            header.trim().to_owned()
        })
        .collect()
}

fn header_lookup(headers: &[String]) -> HashMap<String, usize> {
    let mut lookup = HashMap::new();
    for (index, header) in headers.iter().enumerate() {
        if !is_junk_header(header) {
            lookup.entry(header.to_ascii_lowercase()).or_insert(index);
        }
    }
    lookup
}

fn required_column(lookup: &HashMap<String, usize>, name: &str, path: &Path) -> AppResult<usize> {
    lookup
        .get(&name.to_ascii_lowercase())
        .copied()
        .ok_or_else(|| {
            AppError::Message(format!(
                "Required column '{name}' is missing from {}",
                path.display()
            ))
        })
}

fn required_auto_columns(group: &AutoChannelGroup) -> Vec<String> {
    let mut columns = vec!["Date".to_owned(), "Time".to_owned()];
    for phase in &group.phases {
        columns.extend([
            format!("Uac-{phase}"),
            format!("Iac-{phase}"),
            format!("P-{phase}"),
            format!("Q-{phase}"),
            format!("S-{phase}"),
            format!("PF-{phase}"),
        ]);
    }
    columns.extend([
        format!("Iac-{}", group.total),
        format!("P-{}", group.total),
        format!("Q-{}", group.total),
        format!("S-{}", group.total),
        format!("PF-{}", group.total),
        format!("FreqU-{}", group.phases[0]),
    ]);
    columns
}

fn raw_value<'a>(record: &'a StringRecord, lookup: &HashMap<String, usize>, name: &str) -> &'a str {
    lookup
        .get(&name.to_ascii_lowercase())
        .and_then(|index| record.get(*index))
        .unwrap_or_default()
        .trim()
}

fn parse_optional_number(
    raw: &str,
    path: &Path,
    source_row: usize,
    column: &str,
) -> AppResult<Option<f64>> {
    let raw = raw.trim();
    if raw.is_empty()
        || raw.eq_ignore_ascii_case("nan")
        || raw.chars().all(|character| character == '-')
    {
        return Ok(None);
    }
    let value = raw.parse::<f64>().map_err(|_| {
        AppError::Message(format!(
            "Invalid numeric value '{raw}' in column '{column}' at {} row {source_row}",
            path.display()
        ))
    })?;
    if !value.is_finite() {
        return Ok(None);
    }
    Ok(Some(value))
}

fn should_skip_record(record: &StringRecord) -> bool {
    record.iter().all(|value| value.trim().is_empty())
        || record
            .get(0)
            .is_some_and(|value| value.trim().eq_ignore_ascii_case("EOF"))
}

fn is_junk_header(header: &str) -> bool {
    let header = header.trim();
    header.is_empty() || header.chars().all(|character| character == '-')
}

fn parse_meter_timestamp(value: &str) -> Option<NaiveDateTime> {
    [
        "%m/%d/%Y %I:%M:%S %p",
        "%-m/%-d/%Y %-I:%M:%S %p",
        "%Y-%m-%d %H:%M:%S",
    ]
    .iter()
    .find_map(|format| NaiveDateTime::parse_from_str(value, format).ok())
}

fn parse_auto_timestamp(date: &str, time: &str) -> Option<NaiveDateTime> {
    NaiveDateTime::parse_from_str(&format!("{date} {time}"), "%Y/%m/%d %H:%M:%S").ok()
}

fn format_meter_timestamp(value: NaiveDateTime) -> String {
    let hour = value.hour();
    let display_hour = hour % 12;
    format!(
        "{}/{}/{} {}:{:02}:{:02} {}",
        value.month(),
        value.day(),
        value.year(),
        if display_hour == 0 { 12 } else { display_hour },
        value.minute(),
        value.second(),
        if hour < 12 { "AM" } else { "PM" }
    )
}

fn scale(value: Option<f64>, factor: f64) -> Option<f64> {
    value.map(|value| value * factor)
}

fn average_three(values: [Option<f64>; 3]) -> Option<f64> {
    let [Some(a), Some(b), Some(c)] = values else {
        return None;
    };
    Some((a + b + c) / 3.0)
}

fn sum_three(values: [Option<f64>; 3]) -> Option<f64> {
    let [Some(a), Some(b), Some(c)] = values else {
        return None;
    };
    Some(a + b + c)
}

/// Fallback when Auto Q-* is missing/NAN: magnitude-only triangle Q from S and P.
/// Returns None if the power triangle is materially invalid (|P| > |S| beyond rounding).
fn reactive_power(apparent: Option<f64>, real: Option<f64>) -> Option<f64> {
    let (Some(apparent), Some(real)) = (apparent, real) else {
        return None;
    };
    let ss = apparent * apparent;
    let pp = real * real;
    let residual = ss - pp;
    if residual >= 0.0 {
        return Some(residual.sqrt());
    }
    // Allow tiny floating-point overshoot of |P| past |S|; otherwise N/A.
    let scale = ss.abs().max(1.0);
    if residual.abs() <= scale * 1.0e-6 {
        Some(0.0)
    } else {
        None
    }
}

fn ratio(numerator: Option<f64>, denominator: Option<f64>) -> Option<f64> {
    let (Some(numerator), Some(denominator)) = (numerator, denominator) else {
        return None;
    };
    if denominator.abs() <= NEAR_ZERO {
        None
    } else {
        Some(numerator / denominator)
    }
}

fn neutral_current(values: [Option<f64>; 3]) -> Option<f64> {
    let [Some(a), Some(b), Some(c)] = values else {
        return None;
    };
    Some(a.max(b).max(c) - a.min(b).min(c))
}

fn unbalance(values: [Option<f64>; 3]) -> Option<f64> {
    let [Some(a), Some(b), Some(c)] = values else {
        return None;
    };
    let average = (a + b + c) / 3.0;
    if average.abs() <= NEAR_ZERO {
        return None;
    }
    let maximum_deviation = (a - average)
        .abs()
        .max((b - average).abs())
        .max((c - average).abs());
    Some(maximum_deviation / average.abs() * 100.0)
}
