use log::{Record, LevelFilter, Metadata};

pub fn init(max_level: LevelFilter) {
    log::set_logger(&ZEPHYR_LOGGER).unwrap();
    log::set_max_level(max_level);
}

static ZEPHYR_LOGGER: ZephyrLogger = ZephyrLogger;
struct ZephyrLogger;

impl log::Log for ZephyrLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= log::max_level()
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            println!("{} {}: {}", record.level(), record.target(), record.args());
        }
    }

    fn flush(&self) {
    }
}
