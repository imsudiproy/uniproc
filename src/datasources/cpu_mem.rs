use sysinfo::{Process, System};
use procfs;

pub fn slow_all_process () {
    let me = procfs::process::Process::myself().unwrap();
    let me_stat = me.stat().unwrap();
    let tps = procfs::ticks_per_second();

    println!("{: >5} {: <8} {: >8} {}", "PID", "TTY", "TIME", "CMD");

    let tty = format!("pty/{}", me_stat.tty_nr().1);
    for prc in procfs::process::all_processes().unwrap() {
        let prc = prc.unwrap();
        let stat = prc.stat().unwrap();
        if stat.tty_nr == me_stat.tty_nr {
            // total_time is in seconds
            let total_time =
                (stat.utime + stat.stime) as f32 / (tps as f32);
            println!(
                "{: >5} {: <8} {: >8} {}",
                stat.pid, tty, total_time, stat.comm
            );
        }
    }
}

pub fn monitor_pid(pid : u32){
    
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
}


