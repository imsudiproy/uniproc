//todo: 
//1. Display memory usage in mb
//2. add TUI

use core::time;
use std::thread;

use sysinfo::{System, Pid};

pub fn get_process_info(pid : u32) -> Option<(f32, u64)> {
    let mut system = System::new_all();
    system.refresh_all();

    if let Some(process) = system.process(Pid::from(pid as usize)) {
        let cpu_usage = process.cpu_usage();
        let memory = process.memory();
        Some((cpu_usage, memory))
    } else {
        None
    }
}

pub fn show_all_process (pid: u32, interval: u64, duration: Option<u64>) {
    let start_time = std::time::Instant::now();
    let duration = duration.unwrap_or(u64::MAX);

    loop {
        //refresh system information
        if let Some((cpu, mem)) = get_process_info(pid){
            println!("Monitoring PID: {}", pid);
            println!("CPU usage: {:.2}%, Memory: {} KB", cpu, mem);
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
