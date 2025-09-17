//This file calculates the CPU and Memory usage of a process
use std::io::{self, Write};
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

//Currently working on this
fn find_process_by_name(name: Option<String>) -> Vec<Pid> {
    let mut matches = Vec::new();
    let mut system = System::new_all();
    system.refresh_all();
    
    if let Some(target_name) = name {
        for(pid, process) in system.processes() {
            if let Some(process_name) = process.name().to_str() {
                if process_name == target_name {
                    matches.push(*pid);
                }
            }
        }
    }
    matches
}

//currently working on this
pub fn show_process_by_name (name : Option<String>, interval: u64, duration: Option<u64>) {
    let pids = find_process_by_name(name);

    let mut system = System::new_all();
    system.refresh_all();

    if pids.is_empty() {
        println!("No matching process found!!");
        return;
    }

    println!("Matching Processes: ");
    for (index,pid) in pids.iter().enumerate() {
        if let Some(proc) = system.process(*pid) {   
        println!(
                "[{}] PID: {} | Name: {:?} | CPU: {:.2}% | Memory: {} KB",
                index,
                pid,
                proc.name(),
                proc.cpu_usage(),
                proc.memory()
            );
        }
    }

    print!("Select a process by number: ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_ok() {
        if let Ok(selected_index) = input.trim().parse::<usize>() {
            if let Some(selected_pid) = pids.get(selected_index) {
                show_process_by_pid(selected_pid.as_u32(), interval, duration);
            }
        }
    }

}

pub fn show_process_by_pid (pid: u32, interval: u64, duration: Option<u64>) {
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
