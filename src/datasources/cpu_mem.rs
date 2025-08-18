use core::time;
use std::thread;

use sysinfo::{System, Pid};

pub fn show_all_process (pid: u32, interval: u64, duration: Option<u64>) {
    let mut system = System::new_all();
    let start_time = std::time::Instant::now();
    let duration = duration.unwrap_or(u64::MAX);

    loop {
        //refresh system information
        system.refresh_all();
        if let Some(process) = system.process(Pid::from(pid as usize)){
            let cpu_usage = process.cpu_usage();
            let memory = process.memory();
            println!("Monitoring PID: {}", pid);
            println!("CPU usage: {:.2}%, Memory: {} KB", cpu_usage, memory);
        } else {
            println!("The process PID {} not found", pid);
            break;
        }

        if start_time.elapsed().as_secs() >= duration {
            break;
        }

        thread::sleep(time::Duration::from_millis(interval));

    }
}
