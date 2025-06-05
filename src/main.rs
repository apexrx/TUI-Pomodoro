use cursive::view::Nameable;
use cursive::views::{Dialog, TextView, LinearLayout};
use cursive::{Cursive, CursiveExt};

use std::fmt::format;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

fn main() {
    let mut siv = Cursive::default();

    // Variables
    // 25 min pomodoro duration
    let pomodoro_duration = Arc::new(Mutex::new(25 * 60));
    // 5 min break duration
    let break_duration = Arc::new(Mutex::new(5 * 60));
    // Timer to countdown
    let timer_counter = Arc::new(Mutex::new(*pomodoro_duration.lock().unwrap()));

    // Flags
    let is_break_time = Arc::new(Mutex::new(false));
    let is_running = Arc::new(Mutex::new(false));

    // Cloning the variables to pass into threads where they will accessed and modified.
    let timer_counter_clone = Arc::clone(&timer_counter);
    let is_break_time_clone = Arc::clone(&is_break_time);
    let is_running_clone = Arc::clone(&is_running);

    // TextView for status.
    let status_view = TextView::new(format!(
        "Status: {}",
        format_status(*is_break_time.lock().unwrap(), *is_running.lock().unwrap())
    ))
    .with_name("status");

    // TextView to display time left on the timer, formated as MM::SS.
    let text_view = TextView::new(format!(
        "Time: {}",
        format_time(*timer_counter.lock().unwrap())
    ))
    .with_name("timer");

    // Linear Layout for both Text and Status view.
    let linear_layout = LinearLayout::vertical()
        .child(text_view)
        .child(status_view);

    // Main UI
    siv.add_layer(
        Dialog::around(linear_layout)
            // +1 min button
            .button("+1 min", {
                let pomodoro_duration = Arc::clone(&pomodoro_duration);
                let timer_counter = Arc::clone(&timer_counter);
                move |s| {
                    let mut duration = pomodoro_duration.lock().unwrap();
                    *duration += 60;

                    let mut timer_value = timer_counter.lock().unwrap();
                    *timer_value = *duration;

                    s.call_on_name("timer", |view: &mut TextView| {
                        view.set_content(format!("Time: {}", format_time(*duration)));
                    });
                }
            })
            // -1 min button
            .button("-1 min", {
                let pomodoro_duration = Arc::clone(&pomodoro_duration);
                let timer_counter = Arc::clone(&timer_counter);
                move |s| {
                    let mut duration = pomodoro_duration.lock().unwrap();
                    if *duration > 60 {
                        *duration -= 60;
                    }

                    let mut timer_value = timer_counter.lock().unwrap();
                    *timer_value = *duration;

                    s.call_on_name("timer", |view: &mut TextView| {
                        view.set_content(format!("Time: {}", format_time(*duration)));
                    });
                }
            })
            // Start/Stop button
            .button("Start/Stop", {
                let is_running_clone = Arc::clone(&is_running);
                move |s| {
                    let mut running = is_running_clone.lock().unwrap();
                    *running = !*running;

                    if *running {
                        s.call_on_name("timer", |view: &mut TextView| {
                            view.set_content("Timer is running...");
                        });
                    } else {
                        s.call_on_name("timer", |view: &mut TextView| {
                            view.set_content("Timer is paused.");
                        });
                    }
                }
            })
            .button("Reset", {
                let is_running_clone = Arc::clone(&is_running);
                let pomodoro_duration = Arc::clone(&pomodoro_duration);
                let timer_counter = Arc::clone(&timer_counter);
                let is_break_time_clone = Arc::clone(&is_break_time);

                move |s| {
                    let mut running = is_running_clone.lock().unwrap();
                    *running = false;

                    let mut on_break = is_break_time_clone.lock().unwrap();
                    *on_break = false;

                    let mut duration = pomodoro_duration.lock().unwrap();
                    let mut timer_value = timer_counter.lock().unwrap();

                    *timer_value = *duration;

                    s.call_on_name("timer", |view: &mut TextView| {
                       view.set_content(format!("Time: {}", format_time(*timer_value)))
                    });
                }
            })
            .button("Quit", |s| s.quit())
            .title("Pomodoro Timer"),
    );

    // First thread for handling the countdown of the timer.
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            let mut time_left = timer_counter_clone.lock().unwrap();
            let mut break_time = is_break_time_clone.lock().unwrap();
            let running = is_running_clone.lock().unwrap();

            if *running {
                if *time_left > 0 {
                    *time_left -= 1;
                } else {
                    if *break_time {
                        *time_left = *pomodoro_duration.lock().unwrap();
                        *break_time = false;
                    } else {
                        *time_left = *break_duration.lock().unwrap();
                        *break_time = true;
                    }
                }
            }
        }
    });

    // Second thread to refresh the UI every second.
    let cb_sink = siv.cb_sink().clone();
    let timer_counter_for_refresh = Arc::clone(&timer_counter);
    let is_break_time_for_refresh = Arc::clone(&is_break_time);
    let is_running_time_for_refresh = Arc::clone(&is_running);

    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(1));

            let timer_counter_refresh = Arc::clone(&timer_counter_for_refresh);
            let is_break_time_refresh = Arc::clone(&is_break_time_for_refresh);
            let is_running_time_refresh = Arc::clone(&is_running_time_for_refresh);

            cb_sink
                .send(Box::new(move |s| {
                    let time_left = *timer_counter_refresh.lock().unwrap();
                    let is_break = *is_break_time_refresh.lock().unwrap();
                    let is_run = *is_running_time_refresh.lock().unwrap();
                    // Refreshing the timer.
                    s.call_on_name("timer", |view: &mut TextView| {
                        view.set_content(format!("Time: {}", format_time(time_left)));
                    });
                    // Refreshing the status.
                    s.call_on_name("status", |view: &mut TextView| {
                       view.set_content(format!("Status: {}", format_status(is_break, is_run)));
                    });
                }))
                .unwrap();
        }
    });

    siv.run();
}

// Helper function to display time in format MM:SS.
fn format_time(time: usize) -> String {
    let minutes = time / 60;
    let seconds = time % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

fn format_status(status: bool, status2: bool) -> String {
    if !status2 {
        format!("Timer is paused")
    } else {
        if status {
            format!("On Break!")
        } else {
            format!("Working!")
        }
    }
}
