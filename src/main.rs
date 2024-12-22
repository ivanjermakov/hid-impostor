use std::{any::Any, fs::canonicalize, path::PathBuf};

use anyhow::{Context, Result};
use clap::{value_parser, Parser};
use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AbsInfo, AbsoluteAxisCode, AbsoluteAxisEvent, AttributeSet, BusType, Device,
    EventSummary::{self, *},
    InputId, KeyCode, UinputAbsSetup,
};

#[derive(Parser)]
#[command(author, version, about = "Create a virtual HID device from a physical HID device", long_about = None)]
struct Args {
    #[arg(value_parser = value_parser!(PathBuf))]
    path: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let path = canonicalize(&args.path).context(format!("no device {}", args.path.display()))?;
    println!("input device is {}", path.display());

    let mut device = Device::open(path)?;
    println!("{:?}", device.input_id());

    let mut virt_device = make_virt_device()?;
    for path in virt_device.enumerate_dev_nodes_blocking()? {
        println!("virt device available as {}", path?.display());
    }

    loop {
        for ev in device.fetch_events()? {
            match ev.destructure() {
                Synchronization(..) => continue,
                Key(key_event, key_code, _) => println!("{:?} {:?}", key_event, key_code),
                AbsoluteAxis(event, code, value) => {
                    println!("{:?} {:?} {}", event.event_type(), code, value);
                    let virt_ev = *AbsoluteAxisEvent::new(code, value);
                    virt_device.emit(&[virt_ev])?;
                }
                evs => println!("other {:?}", EventSummary::type_id(&evs)),
            }
        }
    }
}

fn make_virt_device() -> Result<VirtualDevice> {
    let xbox_id = InputId::new(BusType::BUS_USB, 0x45e, 0x28e, 0x101);
    let xbox_name = "Microsoft X-Box 360 pad";
    let mut keys = AttributeSet::<KeyCode>::new();
    for key in [
        KeyCode::BTN_SOUTH,
        KeyCode::BTN_EAST,
        KeyCode::BTN_NORTH,
        KeyCode::BTN_WEST,
        KeyCode::BTN_TL,
        KeyCode::BTN_TR,
        KeyCode::BTN_SELECT,
        KeyCode::BTN_START,
        KeyCode::BTN_MODE,
        KeyCode::BTN_THUMBL,
        KeyCode::BTN_THUMBR,
    ] {
        keys.insert(key);
    }
    let axis_max = 256;
    let abs_setup = AbsInfo::new(axis_max / 2, 0, axis_max, 0, 0, 1);
    let virt_device = VirtualDeviceBuilder::new()?
        .name(xbox_name)
        .input_id(xbox_id)
        .with_keys(&keys)?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_X, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Y, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_Z, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_RX, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_RY, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_RZ, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_HAT0X, abs_setup))?
        .with_absolute_axis(&UinputAbsSetup::new(AbsoluteAxisCode::ABS_HAT0Y, abs_setup))?
        .build()?;
    Ok(virt_device)
}
