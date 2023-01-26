use flexi_logger::{DeferredNow, Duplicate, FileSpec};

pub fn initialize() -> () {
    // log to file and stderr
    flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory("/tmp/astria-rv-rs"))
        .format(
            |w: &mut dyn std::io::Write,
             now: &mut DeferredNow,
             record: &log::Record| {
                write!(
                    w,
                    "{} [{}] {}",
                    now.format("%Y-%m-%d %H:%M:%S%.6f"),
                    record.level(),
                    &record.args()
                )
            },
        )
        .duplicate_to_stderr(Duplicate::All)
        .append()
        .start()
        .unwrap();
}