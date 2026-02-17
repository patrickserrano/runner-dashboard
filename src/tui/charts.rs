//! Chart helpers for the metrics panel
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]

use ratatui::{
    style::{Color, Style},
    symbols,
    widgets::{Bar, BarChart, BarGroup, Block, Borders, Gauge},
};

/// Create a simple bar chart for duration distribution
pub fn duration_bar_chart<'a>(
    buckets: &[(String, u32)],
    title: &'a str,
) -> BarChart<'a> {
    let bars: Vec<Bar> = buckets
        .iter()
        .map(|(label, count)| {
            Bar::default()
                .label(label.clone())
                .value(u64::from(*count))
                .style(Style::default().fg(Color::Cyan))
        })
        .collect();

    let group = BarGroup::default().bars(&bars);

    BarChart::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title),
        )
        .data(group)
        .bar_width(8)
        .bar_gap(1)
        .bar_style(Style::default().fg(Color::Cyan))
        .value_style(Style::default().fg(Color::White))
}

/// Create a gauge for success rate or uptime
pub fn rate_gauge(
    rate: f64,
    label: &str,
) -> Gauge<'_> {
    let color = if rate >= 90.0 {
        Color::Green
    } else if rate >= 70.0 {
        Color::Yellow
    } else {
        Color::Red
    };

    Gauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(label),
        )
        .gauge_style(Style::default().fg(color))
        .percent(rate.clamp(0.0, 100.0) as u16)
        .label(format!("{rate:.1}%"))
}

/// Create a simple sparkline-style string from values
pub fn mini_sparkline(values: &[u32], width: usize) -> String {
    if values.is_empty() {
        return "-".repeat(width);
    }

    let max_val = *values.iter().max().unwrap_or(&1);
    if max_val == 0 {
        return symbols::bar::HALF.to_string().repeat(width.min(values.len()));
    }

    let bars = [
        symbols::bar::ONE_EIGHTH,
        symbols::bar::ONE_QUARTER,
        symbols::bar::THREE_EIGHTHS,
        symbols::bar::HALF,
        symbols::bar::FIVE_EIGHTHS,
        symbols::bar::THREE_QUARTERS,
        symbols::bar::SEVEN_EIGHTHS,
        symbols::bar::FULL,
    ];

    values
        .iter()
        .take(width)
        .map(|&v| {
            let normalized = (f64::from(v) / f64::from(max_val) * 7.0).round() as usize;
            bars[normalized.min(7)]
        })
        .collect()
}

/// Format a count with a visual bar
pub fn count_with_bar(count: u32, max_count: u32, bar_width: usize) -> String {
    let filled = if max_count > 0 {
        ((f64::from(count) / f64::from(max_count)) * bar_width as f64).round() as usize
    } else {
        0
    };

    let bar: String = std::iter::repeat_n(symbols::block::FULL, filled)
        .chain(std::iter::repeat_n(symbols::block::ONE_EIGHTH, bar_width - filled))
        .collect();

    format!("{bar} {count:>4}")
}
