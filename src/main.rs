use std::{
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
    thread::sleep,
    time::Duration,
};

use anyhow::{Context, Result};
use clap::{value_parser, Parser};

extern crate hidapi;

#[derive(Parser)]
#[command(name = "path-parser")]
#[command(about = "A simple CLI that parses a file path argument", long_about = None)]
struct Args {
    #[arg(value_parser = value_parser!(PathBuf))]
    path: PathBuf,
}

#[derive(Debug)]
struct HidId {
    vendor: u16,
    product: u16,
}

fn main() -> Result<()> {
    let Args { path } = Args::try_parse()?;

    let path = canonicalize(&path).context(format!("no device {path:?}"))?;
    println!("path {path:?}");

    let id = find_id(&path).context(format!("unable to extract id {path:?}"))?;
    println!("id {id:?}");

    let hid_api = hidapi::HidApi::new()?;
    let device = hid_api
        .open(id.vendor, id.product)
        .context("cannot open device")?;

    let mut rd_buffer = [0u8; 1 << 12];
    let rd_size = device.get_report_descriptor(&mut rd_buffer)?;
    let rd_buffer = rd_buffer[..rd_size].to_vec();
    print_hex(&rd_buffer);

    let mut buffer = [0u8; 12];
    loop {
        match device.read(&mut buffer) {
            Ok(0) => break,
            Ok(bytes_read) => {
                print_hex(&buffer[..bytes_read]);
            }
            Err(e) => eprintln!("Error reading from device: {}", e),
        }
        sleep(Duration::from_millis(10));
    }

    Ok(())
}

fn find_id(dev_path: &Path) -> Result<HidId> {
    let device_name = dev_path
        .strip_prefix("/dev/input/")
        .context("incorrect prefix")?
        .to_string_lossy();

    let sysfs_path = format!("/sys/class/input/{}/device", device_name);
    let vendor_path = format!("{sysfs_path}/id/vendor");
    let product_path = format!("{sysfs_path}/id/product");
    Ok(HidId {
        vendor: u16::from_str_radix(
            read_to_string(vendor_path)
                .context("no vendor file")?
                .trim(),
            16,
        )?,
        product: u16::from_str_radix(
            read_to_string(product_path)
                .context("no product file")?
                .trim(),
            16,
        )?,
    })
}

fn print_hex(data: &[u8]) {
    for byte in data {
        print!("{:02X} ", byte);
    }
    println!();
}
