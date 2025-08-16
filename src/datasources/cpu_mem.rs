use sysinfo::{
    Components, Disks, Networks, System,
};

pub fn print_system_info(){
    let mut sys = System::new_all();
    sys.refresh_all();

    println!("=> system:");
// RAM and swap information:
    println!("total memory: {} bytes", sys.total_memory());
    println!("used memory : {} bytes", sys.used_memory());
    println!("total swap  : {} bytes", sys.total_swap());
    println!("used swap   : {} bytes", sys.used_swap());

    // Display system information:
    println!("System name:             {:?}", System::name());
    println!("System kernel version:   {:?}", System::kernel_version());
    println!("System OS version:       {:?}", System::os_version());
    println!("System host name:        {:?}", System::host_name());

// Number of CPUs:
println!("NB CPUs: {}", sys.cpus().len());
}


