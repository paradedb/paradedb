use crate::runner::{ConnInfo, JobRunner, RuntimeStats, SuiteRunner};
use crate::table_helper::ArbitraryTableRow;
use anyhow::Error;
use cursive_core::align::HAlign;
use cursive_core::event::Event;
use cursive_core::style::{BaseColor, Color, ColorStyle, ColorType, PaletteColor};
use cursive_core::traits::{Nameable, Resizable};
use cursive_core::utils::markup::StyledString;
use cursive_core::view::SizeConstraint;
use cursive_core::views::{LinearLayout, NamedView, Panel, ResizedView, TextView};
use cursive_core::Cursive;
use cursive_multiplex::Mux;
use cursive_table_view::TableView;
use human_bytes::human_bytes;
use humantime::format_duration;
use postgres::Row;
use std::fmt::Display;
use std::num::NonZeroU32;
use std::ops::DerefMut;
use std::panic::panic_any;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Standard TUI mode
pub fn run(suite_runner: Arc<SuiteRunner>) -> anyhow::Result<()> {
    let mut cursive = cursive::default();

    let theme = {
        let mut t = cursive.current_theme().clone();
        t.palette[PaletteColor::View] = Color::TerminalDefault;
        t.palette[PaletteColor::Primary] = Color::TerminalDefault;
        t.palette[PaletteColor::Secondary] = Color::TerminalDefault;
        t.palette[PaletteColor::Tertiary] = Color::TerminalDefault;
        t
    };
    cursive.set_theme(theme);

    let mut mux = Mux::new();
    let mut root_id = mux.root().build().unwrap();

    // Monitor job
    let mut mon_view = LinearLayout::vertical();
    for monitor in suite_runner.monitor_runners() {
        make_runner_view(&suite_runner, &mut mon_view, &monitor);
    }
    root_id = mux.add_below(mon_view, root_id)?;

    // All other jobs
    let mut job_views = LinearLayout::vertical();
    for job in suite_runner.runners() {
        make_runner_view(&suite_runner, &mut job_views, &job);
    }
    mux.add_right_of(job_views, root_id)?;
    mux.set_container_split_ratio(root_id, 0.5)?;

    cursive.add_fullscreen_layer(
        LinearLayout::vertical().child(mux).child(
            TextView::new("initializing...")
                .with_name("status")
                .full_width(),
        ),
    );

    let mut started: Option<Instant> = None;

    {
        const RESERVED_KEYS: [char; 4] = ['q', '+', '-', ' '];
        let sr = suite_runner.clone();
        for runner in sr.runners() {
            if let Some(keycode) = runner.job().cancel_keycode {
                if RESERVED_KEYS.contains(&keycode) {
                    panic!("keycode `{}` is reserved", keycode);
                }

                let cancel_every = runner.job().cancel_every;
                let runner_for_callback = runner.clone();
                let runner_for_auto = runner.clone();

                cursive.add_global_callback(keycode, move |_c| {
                    runner_for_callback.cancel_query();
                });

                if let Some(cancel_every) = cancel_every {
                    let runner = runner_for_auto;
                    let running = Arc::new(AtomicBool::new(true));
                    let running_clone = running.clone();

                    thread::spawn(move || {
                        while running_clone.load(Ordering::Relaxed) {
                            thread::sleep(Duration::from_secs_f64(cancel_every));
                            if running_clone.load(Ordering::Relaxed) {
                                runner.cancel_query();
                            }
                        }
                    });

                    // Store the running flag so we can stop it when the TUI exits
                    cursive.set_user_data(running);
                }
            }

            if let Some(keycode) = runner.job().pause_keycode {
                if RESERVED_KEYS.contains(&keycode) {
                    panic!("keycode `{}` is reserved", keycode);
                }

                let runner = runner.clone();
                cursive.add_global_callback(keycode, move |_c| {
                    runner.toggle_pause();
                });
            }
        }
        cursive.add_global_callback('q', move |c| {
            sr.terminate();
            let _ = sr.wait_for_finish();
            c.quit();
        });
    }
    {
        let sr = suite_runner.clone();
        cursive.add_global_callback(' ', move |_| {
            sr.toggle_pause();
        });
    }
    cursive.add_global_callback('+', move |c| {
        if let Some(fps) = c.fps() {
            c.set_fps(fps.get().saturating_add(1));
        } else {
            c.set_fps(1);
        }
    });
    cursive.add_global_callback('-', move |c| {
        if let Some(fps) = c.fps() {
            let new_fps = fps.get().saturating_sub(1).max(1);
            c.set_fps(new_fps);
        } else {
            c.set_fps(1);
        }
    });

    {
        let sr = suite_runner.clone();
        cursive.add_global_callback(Event::Refresh, move |c| {
            if !sr.paused() && started.is_none() {
                started = Some(Instant::now());
            }
            // refresh all job views
            sr.runners()
                .for_each(|runner| refresh_runner_view(c, &sr, &runner));
            sr.monitor_runners()
                .for_each(|runner| refresh_runner_view(c, &sr, &runner));

            let mut tv = c.find_name::<TextView>("status").unwrap();
            tv.set_content(format!(
                "{}: {}   +/- to change UI refresh interval ({}fps)  `q`=quit   {}    {}",
                sr.name(),
                if sr.errored() {
                    "Errored"
                } else if sr.paused() {
                    "Paused"
                } else {
                    "Running"
                },
                c.fps().unwrap_or_else(|| NonZeroU32::new(1).unwrap()),
                sr.pgver(),
                if let Some(duration) = sr.first_error_duration() {
                    format!("ERROR after={}", format_duration(duration))
                } else {
                    started
                        .as_ref()
                        .map(|st| format!("runtime={}", format_duration(st.elapsed())))
                        .unwrap_or_else(|| "<not started>".to_string())
                }
            ));
        });
    }

    cursive.set_autorefresh(true);
    cursive.run();
    Ok(())
}

type ResultsTable = TableView<ArbitraryTableRow, usize>;

fn make_runner_view(sr: &Arc<SuiteRunner>, parent: &mut LinearLayout, runner: &Arc<JobRunner>) {
    let mut table_list = LinearLayout::vertical();
    for (conninfo, runtime_stats) in runner.runtime_stats() {
        if runner.is_select() {
            let results_table = ResultsTable::new()
                .with_name(runner.view_id(&conninfo, "results"))
                .resized(
                    SizeConstraint::Full,
                    if runner.is_monitor() {
                        SizeConstraint::Full
                    } else {
                        SizeConstraint::AtLeast(runner.job().window_height.unwrap_or(3))
                    },
                );

            let panel = Panel::new(results_table)
                .title(style_title(sr, runner, &conninfo, &runtime_stats))
                .with_name(runner.view_id(&conninfo, "panel"));
            table_list.add_child(panel);
            table_list.add_child(TextView::new("\n").with_name(runner.view_id(&conninfo, "error")));
        } else {
            let view = TextView::new(style_title(sr, runner, &conninfo, &runtime_stats))
                .with_name(runner.view_id(&conninfo, "text-view"));
            table_list.add_child(view);
            table_list.add_child(TextView::new("").with_name(runner.view_id(&conninfo, "error")));
        }
    }

    let job_panel = Panel::new(table_list.min_height(runner.job().destinations.len()))
        .title_position(HAlign::Left)
        .title(StyledString::styled(
            runner.title(),
            Color::Light(BaseColor::White),
        ));

    parent.add_child(job_panel);
}

fn style_error(error: &dyn Display) -> StyledString {
    StyledString::styled(
        format!("{error}\n\n"),
        ColorStyle {
            front: ColorType::Color(Color::Light(BaseColor::White)),
            back: ColorType::Color(Color::Light(BaseColor::Red)),
        },
    )
}

fn refresh_runner_view(siv: &mut Cursive, sr: &Arc<SuiteRunner>, runner: &Arc<JobRunner>) {
    for (conninfo, runtime_stats) in runner.runtime_stats() {
        if let Some(worker_error) = runner.worker_error() {
            update_error_view(siv, &runner, &conninfo, worker_error);
        } else {
            let panel = siv.find_name::<Panel<ResizedView<NamedView<ResultsTable>>>>(
                &runner.view_id(&conninfo, "panel"),
            );

            if let Some(mut panel) = panel {
                panel.set_title(style_title(sr, runner, &conninfo, &runtime_stats));
            }

            let text_view = siv.find_name::<TextView>(&runner.view_id(&conninfo, "text-view"));
            if let Some(mut text_view) = text_view {
                text_view.set_content(style_title(sr, runner, &conninfo, &runtime_stats));
            }

            let results_table =
                siv.find_name::<ResultsTable>(&runner.view_id(&conninfo, "results"));
            if let Some(results_table) = results_table {
                match runtime_stats.results {
                    Ok(_) if runtime_stats.assert_error.is_some() => {
                        if let Some(mut error_view) =
                            siv.find_name::<TextView>(&runner.view_id(&conninfo, "error"))
                        {
                            error_view.set_content(style_error(
                                &runtime_stats.assert_error.clone().unwrap(),
                            ));
                        } else {
                            panic_any(runtime_stats.assert_error.clone().unwrap());
                        }
                    }
                    Ok(rows) => make_results_view(rows, results_table),

                    Err(e) => {
                        update_error_view(siv, &runner, &conninfo, Arc::new(anyhow::anyhow!(e)))
                    }
                }
            } else if let Err(e) = runtime_stats.results {
                update_error_view(siv, &runner, &conninfo, Arc::new(anyhow::anyhow!(e)));
            }
        }
    }
}

fn update_error_view(
    siv: &mut Cursive,
    runner: &&Arc<JobRunner>,
    conninfo: &ConnInfo,
    worker_error: Arc<Error>,
) {
    if let Some(mut error_view) = siv.find_name::<TextView>(&runner.view_id(conninfo, "error")) {
        error_view.set_content(style_error(&worker_error));
    } else {
        panic_any(worker_error);
    }
}

fn panel_title(runner: &JobRunner, conninfo: &ConnInfo, runtime_stats: &RuntimeStats) -> String {
    format!(
        "{}: count={} tps={:.4} cpu={:.2}% mem={} pid={} {}",
        conninfo.name(),
        runtime_stats.count,
        runtime_stats.tps(),
        runtime_stats.cpu_usage,
        human_bytes(runtime_stats.mem_usage as f64),
        conninfo.pid(),
        runner
            .job()
            .atomic_connection
            .then_some("(atomic)")
            .unwrap_or_default()
    )
}

fn style_title(
    sr: &Arc<SuiteRunner>,
    runner: &JobRunner,
    conninfo: &ConnInfo,
    runtime_stats: &RuntimeStats,
) -> StyledString {
    let color = if runner.connection_errored(conninfo) {
        Color::Light(BaseColor::Red)
    } else if sr.errored() {
        Color::Dark(BaseColor::White)
    } else if runner.running() {
        conninfo.color()
    } else if (sr.paused() && !runner.is_monitor()) || runner.paused() {
        Color::Light(BaseColor::Yellow)
    } else {
        Color::Dark(BaseColor::White)
    };
    StyledString::styled(panel_title(runner, conninfo, runtime_stats), color)
}

fn make_results_view(rows: Vec<Row>, mut view_ref: cursive_core::views::ViewRef<ResultsTable>) {
    let rows = rows.into_iter().map(ArbitraryTableRow).collect::<Vec<_>>();

    if let Some(first) = rows.first() {
        let mut table_view = TableView::new();
        let col_count = first.0.columns().len();
        for (i, col) in first.0.columns().iter().enumerate() {
            let width = first.col_width(i);
            let align = first.halign(i);

            table_view.add_column(i, col.name(), |c| {
                match width {
                    Some(width) => c.width(width.max(col.name().len() + 4)),
                    _ => c.width_percent(100 / col_count),
                }
                .align(align)
            });
        }
        table_view.set_items(rows);
        table_view.disable();
        *view_ref.deref_mut() = table_view;
    } else {
        view_ref.clear();
    }
}
