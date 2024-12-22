use std::{any::Any, fs::canonicalize, path::PathBuf};

use anyhow::{Context, Result};
use clap::{value_parser, Parser};
use evdev::{
    Device,
    EventSummary::{self, *},
};

mod hex;

#[derive(Parser)]
#[command(name = "path-parser")]
#[command(about = "A simple CLI that parses a file path argument", long_about = None)]
struct Args {
    #[arg(value_parser = value_parser!(PathBuf))]
    path: PathBuf,
}

fn main() -> Result<()> {
    let Args { path } = Args::try_parse()?;

    let path = canonicalize(&path).context(format!("no device {path:?}"))?;
    println!("path {path:?}");

    let mut device = Device::open(path)?;
    println!("{:?}", device.input_id());

    loop {
        for ev in device.fetch_events()? {
            match ev.destructure() {
                Synchronization(..) => continue,
                Key(key_event, key_code, _) => println!("{:?} {:?}", key_event, key_code),
                AbsoluteAxis(event, code, value) => {
                    println!("{:?} {:?} {}", event.event_type(), code, value)
                }
                evs => println!("other {:?}", EventSummary::type_id(&evs)),
            }
        }
    }
}
