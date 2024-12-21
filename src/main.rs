use std::{
    fs::{canonicalize, read_to_string},
    path::{Path, PathBuf},
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
    let path = canonicalize(path)?;
    println!("{:?}", path);
    let id = find_id(&path).context(format!("cannot find device id for {path:?}"))?;
    println!("{:?}", id);

    let hid_api = hidapi::HidApi::new()?;

    let device = hid_api.open(id.vendor, id.product)?;

    let mut buf = [0u8; 8];
    let res = device.read(&mut buf[..])?;
    println!("Read: {:?}", &buf[..res]);

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
