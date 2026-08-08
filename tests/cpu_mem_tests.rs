use std::process;
use sysinfo::System;
use uniproc::datasources::cpu_mem;

#[test]
fn test_sampler_collects_current_process() {
    let mut sampler = cpu_mem::ProcessSampler::new(process::id()).expect("current process exists");
    let sample = sampler.sample().expect("current process can be sampled");
    assert_eq!(sample.pid, process::id());
    assert!(sample.memory_bytes > 0);
    assert!(sample.system_memory_bytes >= sample.memory_bytes);
}
#[test]
//Invalid PID test
fn test_get_process_info_invalid_pid() {
    let mut system = System::new_all();
    system.refresh_all();
    let result = cpu_mem::get_process_info(999999, &mut system);
    assert!(result.is_none());
}
