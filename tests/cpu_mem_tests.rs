use uniproc::datasources::cpu_mem;

#[test]
//Invalid PID test
fn test_get_process_info_invalid_pid() {
    let result = cpu_mem::get_process_info(999999);
    assert!(result.is_none());
}
