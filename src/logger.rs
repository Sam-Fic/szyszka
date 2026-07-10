use std::path::PathBuf;

use directories_next::ProjectDirs;
use file_rotate::compression::Compression;
use file_rotate::suffix::{AppendTimestamp, FileLimit};
use file_rotate::{ContentLimit, FileRotate};
use handsome_logger::{CombinedLogger, ConfigBuilder, FormatText, SharedLogger, TermLogger, TimeFormat, WriteLogger};
use log::{info, LevelFilter, Record};

const APP_NAME: &str = "szyszka";

pub fn get_cache_path() -> Option<PathBuf> {
    ProjectDirs::from("com", "github.samfic", "Szyszka").map(|p| PathBuf::from(p.cache_dir()))
}

pub fn setup_logger() {
    let term_config = ConfigBuilder::default()
        .set_level(LevelFilter::Info)
        .set_message_filtering(Some(filtering_messages))
        .build();

    let file_config = ConfigBuilder::default()
        .set_level(LevelFilter::Debug)
        .set_write_once(true)
        .set_message_filtering(Some(filtering_messages))
        .set_time_format(TimeFormat::DateTimeWithMicro, None)
        .set_format_text(FormatText::DefaultWithThreadFile.get(), None)
        .build();

    let cache_path = get_cache_path();
    let log_path = cache_path.as_ref().map(|p| p.join(format!("{APP_NAME}.log")));

    let write_rotater = log_path.as_ref().and_then(|log_path| {
        let _ = std::fs::create_dir_all(log_path.parent().unwrap_or(std::path::Path::new(".")));
        Some(FileRotate::new(
            log_path,
            AppendTimestamp::default(FileLimit::MaxFiles(3)),
            ContentLimit::BytesSurpassed(10 * 1024 * 1024),
            Compression::None,
            None,
        ))
    });

    let mut loggers: Vec<Box<dyn SharedLogger>> = vec![TermLogger::new_from_config(term_config)];
    if let Some(rotater) = write_rotater {
        loggers.push(WriteLogger::new(file_config, rotater));
    }

    let _ = CombinedLogger::init(loggers);
    if let Some(p) = &log_path {
        info!("Logging to file \"{}\" and terminal", p.display());
    } else {
        info!("Logging to terminal only, file logging is disabled");
    }
}

fn filtering_messages(record: &Record) -> bool {
    record.module_path().is_none_or(|module_path| module_path.starts_with("szyszka"))
}
