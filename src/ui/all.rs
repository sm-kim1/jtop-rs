use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

use crate::app::App;

fn usage_color(pct: f32) -> Color {
    if pct > 80.0 {
        Color::Red
    } else if pct > 50.0 {
        Color::Yellow
    } else {
        Color::Green
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1024 * 1024 * 1024;
    const MB: u64 = 1024 * 1024;
    const KB: u64 = 1024;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

fn format_freq_mhz(hz: u64) -> String {
    if hz == 0 {
        return "N/A".to_string();
    }
    let mhz = hz as f64 / 1_000_000.0;
    if mhz >= 1000.0 {
        format!("{:.1} GHz", mhz / 1000.0)
    } else {
        format!("{:.0} MHz", mhz)
    }
}

pub fn render(frame: &mut Frame, app: &App, area: Rect) {
    // Split into left (60%) and right (40%) columns
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    render_left(frame, app, cols[0]);
    render_right(frame, app, cols[1]);
}

fn render_left(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = &app.stats.cpu;
    let fan = &app.stats.fan;

    // Left: CPU block, Memory block, Fan block
    // Rows: CPU takes most space, memory fixed height, fan fixed height
    let cpu_height = (cpu.cores.len() as u16 + 3).max(5);
    let mem_height = 4u16;
    let fan_height = (fan.fans.len() as u16 + 2).max(3);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(cpu_height),
            Constraint::Length(mem_height),
            Constraint::Length(fan_height),
            Constraint::Min(0),
        ])
        .split(area);

    render_cpu_block(frame, app, rows[0]);
    render_memory_block(frame, app, rows[1]);
    render_fan_block(frame, app, rows[2]);
}

fn render_right(frame: &mut Frame, app: &App, area: Rect) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(5),
            Constraint::Length(5),
            Constraint::Min(0),
        ])
        .split(area);

    render_gpu_block(frame, app, rows[0]);
    render_temp_block(frame, app, rows[1]);
    render_power_block(frame, app, rows[2]);
}

fn render_cpu_block(frame: &mut Frame, app: &App, area: Rect) {
    let cpu = &app.stats.cpu;
    let block = Block::default()
        .title(" CPU ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if cpu.cores.is_empty() {
        let p = Paragraph::new("No CPU data");
        frame.render_widget(p, inner);
        return;
    }

    // Each core gets one line
    let row_count = cpu.cores.len() + 1; // +1 for total line
    let constraints: Vec<Constraint> = (0..row_count).map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, core) in cpu.cores.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let row_area = rows[i];

        if !core.online {
            let line = Line::from(vec![
                Span::styled(
                    format!(" Core {:>2} [offline]", core.id),
                    Style::default().fg(Color::DarkGray),
                ),
            ]);
            frame.render_widget(Paragraph::new(line), row_area);
            continue;
        }

        // Split row: label | gauge | freq
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(12),
                Constraint::Min(10),
                Constraint::Length(10),
            ])
            .split(row_area);

        let label = Paragraph::new(format!(" Core {:>2}", core.id))
            .style(Style::default().fg(Color::White));
        frame.render_widget(label, cols[0]);

        let pct = core.usage.clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .gauge_style(
                Style::default()
                    .fg(usage_color(core.usage))
                    .add_modifier(Modifier::BOLD),
            )
            .percent(pct)
            .label(format!("{:>3}%", pct));
        frame.render_widget(gauge, cols[1]);

        // freq_cur is in kHz for CPU
        let freq_str = if core.freq_cur == 0 {
            "  N/A    ".to_string()
        } else {
            let mhz = core.freq_cur as f64 / 1000.0;
            if mhz >= 1000.0 {
                format!("{:.2}GHz", mhz / 1000.0)
            } else {
                format!("{:.0}MHz ", mhz)
            }
        };
        let freq_label = Paragraph::new(format!(" {}", freq_str))
            .style(Style::default().fg(Color::Cyan));
        frame.render_widget(freq_label, cols[2]);
    }

    // Total line
    if let Some(total_row) = rows.get(cpu.cores.len()) {
        let total_pct = cpu.total_usage.clamp(0.0, 100.0);
        let total_line = Line::from(vec![
            Span::styled(
                format!(" Total: {:.1}%", total_pct),
                Style::default()
                    .fg(usage_color(total_pct))
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        frame.render_widget(Paragraph::new(total_line), *total_row);
    }
}

fn render_memory_block(frame: &mut Frame, app: &App, area: Rect) {
    let mem = &app.stats.memory;
    let block = Block::default()
        .title(" Memory ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Length(1)])
        .split(inner);

    // RAM row
    {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(10)])
            .split(rows[0]);

        frame.render_widget(
            Paragraph::new(" RAM ").style(Style::default().fg(Color::White)),
            cols[0],
        );

        let ram_pct = if mem.ram.total > 0 {
            (mem.ram.used as f64 / mem.ram.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        let label = format!(
            "{} / {} ({:.1}%)",
            format_bytes(mem.ram.used),
            format_bytes(mem.ram.total),
            ram_pct
        );
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(usage_color(ram_pct)))
            .percent(ram_pct.clamp(0.0, 100.0) as u16)
            .label(label);
        frame.render_widget(gauge, cols[1]);
    }

    // SWAP row
    {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(10)])
            .split(rows[1]);

        frame.render_widget(
            Paragraph::new(" SWAP").style(Style::default().fg(Color::White)),
            cols[0],
        );

        let swap_pct = if mem.swap.total > 0 {
            (mem.swap.used as f64 / mem.swap.total as f64 * 100.0) as f32
        } else {
            0.0
        };
        let label = format!(
            "{} / {} ({:.1}%)",
            format_bytes(mem.swap.used),
            format_bytes(mem.swap.total),
            swap_pct
        );
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(usage_color(swap_pct)))
            .percent(swap_pct.clamp(0.0, 100.0) as u16)
            .label(label);
        frame.render_widget(gauge, cols[1]);
    }
}

fn render_fan_block(frame: &mut Frame, app: &App, area: Rect) {
    let fan = &app.stats.fan;
    let block = Block::default()
        .title(" Fan ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Blue));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if fan.fans.is_empty() {
        let p = Paragraph::new(" No fan data").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner);
        return;
    }

    let constraints: Vec<Constraint> = fan.fans.iter().map(|_| Constraint::Length(1)).collect();
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(inner);

    for (i, fan_info) in fan.fans.iter().enumerate() {
        if i >= rows.len() {
            break;
        }
        let row = rows[i];
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(14), Constraint::Min(10)])
            .split(row);

        frame.render_widget(
            Paragraph::new(format!(" {}:", fan_info.name))
                .style(Style::default().fg(Color::White)),
            cols[0],
        );

        let pwm_pct = if fan_info.pwm_max > 0 {
            (fan_info.pwm as f32 / fan_info.pwm_max as f32 * 100.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let label = format!("{:.0}% ({} RPM)", pwm_pct, fan_info.speed);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(Color::Cyan))
            .percent(pwm_pct as u16)
            .label(label);
        frame.render_widget(gauge, cols[1]);
    }
}

fn render_gpu_block(frame: &mut Frame, app: &App, area: Rect) {
    let gpu = &app.stats.gpu;
    let block = Block::default()
        .title(" GPU ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(inner);

    // GPU usage gauge
    {
        let cols = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(10)])
            .split(rows[0]);

        frame.render_widget(
            Paragraph::new(" Load ").style(Style::default().fg(Color::White)),
            cols[0],
        );
        let pct = gpu.usage.clamp(0.0, 100.0) as u16;
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(usage_color(gpu.usage)))
            .percent(pct)
            .label(format!("{pct}%"));
        frame.render_widget(gauge, cols[1]);
    }

    // Frequency line
    let freq_line = Line::from(vec![
        Span::styled(" Freq: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format_freq_mhz(gpu.freq_cur),
            Style::default().fg(Color::Cyan),
        ),
    ]);
    frame.render_widget(Paragraph::new(freq_line), rows[1]);

    // Governor line
    let gov_line = Line::from(vec![
        Span::styled(" Gov:  ", Style::default().fg(Color::Gray)),
        Span::styled(
            gpu.governor.clone(),
            Style::default().fg(Color::White),
        ),
    ]);
    frame.render_widget(Paragraph::new(gov_line), rows[2]);
}

fn render_temp_block(frame: &mut Frame, app: &App, area: Rect) {
    let temp = &app.stats.temperature;
    let block = Block::default()
        .title(" Temperature ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Red));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if temp.zones.is_empty() {
        let p = Paragraph::new(" No temperature data").style(Style::default().fg(Color::DarkGray));
        frame.render_widget(p, inner);
        return;
    }

    // Show up to 3 zones per row compactly
    let zones_per_row = 2usize;
    let mut lines: Vec<Line> = Vec::new();
    let mut chunks: Vec<Span> = Vec::new();

    for (i, zone) in temp.zones.iter().enumerate() {
        let temp_color = if zone.temp > 80.0 {
            Color::Red
        } else if zone.temp > 60.0 {
            Color::Yellow
        } else {
            Color::Green
        };

        // Abbreviate long zone type names
        let name = if zone.name.len() > 8 {
            zone.name[..8].to_string()
        } else {
            zone.name.clone()
        };

        chunks.push(Span::styled(
            format!(" {}: ", name),
            Style::default().fg(Color::Gray),
        ));
        chunks.push(Span::styled(
            format!("{:.0}°C  ", zone.temp),
            Style::default().fg(temp_color).add_modifier(Modifier::BOLD),
        ));

        if (i + 1) % zones_per_row == 0 {
            lines.push(Line::from(chunks.clone()));
            chunks.clear();
        }
    }
    if !chunks.is_empty() {
        lines.push(Line::from(chunks));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}

fn render_power_block(frame: &mut Frame, app: &App, area: Rect) {
    let power = &app.stats.power;
    let block = Block::default()
        .title(" Power ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let total_w = power.total_power / 1000.0;
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(" Total: ", Style::default().fg(Color::Gray)),
        Span::styled(
            format!("{:.1} W", total_w),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ])];

    // Show up to 2 rails per line
    let mut rail_spans: Vec<Span> = Vec::new();
    for (i, rail) in power.rails.iter().enumerate() {
        let rail_w = rail.power / 1000.0;
        // Abbreviate long rail names
        let name = if rail.name.len() > 8 {
            rail.name[..8].to_string()
        } else {
            rail.name.clone()
        };
        rail_spans.push(Span::styled(
            format!(" {}: {:.1}W  ", name, rail_w),
            Style::default().fg(Color::White),
        ));

        if (i + 1) % 2 == 0 {
            lines.push(Line::from(rail_spans.clone()));
            rail_spans.clear();
        }
    }
    if !rail_spans.is_empty() {
        lines.push(Line::from(rail_spans));
    }

    let para = Paragraph::new(lines);
    frame.render_widget(para, inner);
}
