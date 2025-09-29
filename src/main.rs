use clap::{ArgGroup, Parser};
mod datasources;

#[derive(Parser, Debug)]
#[command(name = "uniproc")]
#[command(about = "Monitors process by PID or name", long_about = None)]
#[command(group(
    ArgGroup::new("target")
    .required(true)
    .args(&["pid", "name"])
))]
struct Cli {
    // PID of process to Monitor
    #[arg(required_unless_present = "name")]
    pid: Option<u32>,

    //Process Name (alternate to PID)
    #[arg(long)]
    name: Option<String>,

    //Refresh intervals in miliseconds (default 1000ms)
    #[arg(long, default_value_t = 1000)]
    interval: u64,

    //export in cvs instead of live view
    #[arg(long)]
    csv: Option<String>,

    //export json instead of live view
    #[arg(long)]
    json: Option<String>,

    //duration to run in second
    #[arg(long)]
    duration: Option<u64>,
}

fn main() {
    let cli = Cli::parse();
    //println!("{cli:#?}");

    if let Some(pid) = cli.pid {
        println!("Monitoring PID: {pid}");
        datasources::cpu_mem::show_process_by_pid(pid, cli.interval, cli.duration);
    } else if let Some(name) = cli.name {
        println!("Monitoring Name: {name}");
        datasources::cpu_mem::show_process_by_name(Some(name), cli.interval, cli.duration);
    }

    println!("Refresh interval: {}ms", cli.interval);

    if let Some(csv) = cli.csv {
        println!("Exporting to CVS: {csv}");
        todo!()
    }

    if let Some(json) = cli.json {
        println!("Exporting to json: {json}");
        todo!()
    }

    if let Some(duration) = cli.duration {
        println!("Monitoring for {duration} seconds");
    }
}
