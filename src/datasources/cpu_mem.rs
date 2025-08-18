use core::time;
// use std::os::unix::thread;
// use std::thread::sleep;
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

// pub fn monitor_pid(pid : u32){
    
//     let mut sys = System::new_all();
//     sys.refresh_all();

//     println!("=> system:");
// // RAM and swap information:
//     println!("total memory: {} bytes", sys.total_memory());
//     println!("used memory : {} bytes", sys.used_memory());
//     println!("total swap  : {} bytes", sys.total_swap());
//     println!("used swap   : {} bytes", sys.used_swap());

//     // Display system information:s
//     println!("System name:             {:?}", System::name());
//     println!("System kernel version:   {:?}", System::kernel_version());
//     println!("System OS version:       {:?}", System::os_version());
//     println!("System host name:        {:?}", System::host_name());

// // Number of CPUs:
// println!("NB CPUs: {}", sys.cpus().len());
// }


