use sysinfo::System;
use uniproc::datasources::cpu_mem;
#[test]
//Invalid PID test
fn test_get_process_info_invalid_pid() {
    let mut system = System::new_all();
    system.refresh_all();
    let result = cpu_mem::get_process_info(999999, &mut system);
    assert!(result.is_none());
}
