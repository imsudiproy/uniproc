//This file calculates the CPU and Memory usage of a process
use core::time;
use std::thread;
use sysinfo::{System, Pid};

pub fn get_process_info(pid : u32) -> Option<(f32, u64)> {
    let mut system = System::new_all();
    system.refresh_all();
    //thread::sleep(time::Duration::from_millis(100));
    //system.refresh_all();

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
            println!("CPU usage: {:.2}%, Memory: {} MB", cpu, ((mem as f64)/1024.0));
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


#[cfg(test)]
mod tests {
    use super::*;
    use std::process;

    #[test]
    fn test_get_process_info_current_pid() {
        let pid = process::id();
        let result = get_process_info(pid);
        assert!(result.is_some(), "Expected process info for current PID");
        
        let (cpu, mem) = result.unwrap();
        // Memory should be non-zero for a real process
        assert!(mem > 0, "Memory usage should be greater than 0");
        // CPU usage might be 0 if idle, so no strict check
        println!("CPU: {}, MEM: {} MB", cpu, (mem as f64)/1024.0);
    }

    #[test]
    fn test_get_process_info_invalid_pid() {
        let invalid_pid = u32::MAX; // something that should not exist
        let result = get_process_info(invalid_pid);
        assert!(result.is_none(), "Expected None for invalid PID");
    }
}
