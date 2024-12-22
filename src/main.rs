use std::{collections::HashMap, fs::canonicalize, path::PathBuf};

use anyhow::{Context, Result};
use clap::{value_parser, Parser};
use evdev::{
    uinput::{VirtualDevice, VirtualDeviceBuilder},
    AbsInfo, AbsoluteAxisCode, AbsoluteAxisEvent, AttributeSet, BusType, Device,
    EventSummary::*,
    InputId, KeyCode, UinputAbsSetup,
};

#[derive(Parser)]
#[command(author, version, about = "Create a virtual HID device from a physical HID device", long_about = None)]
struct Args {
    #[arg(value_parser = value_parser!(PathBuf))]
    path: PathBuf,
}

#[derive(Clone, Copy)]
struct Mapping {
    to_code: u16,
    invert: bool,
}

impl Mapping {
    fn from_abs(code: AbsoluteAxisCode) -> Self {
        Self {
            to_code: code.0,
            invert: false,
        }
    }

    fn from_abs_inv(code: AbsoluteAxisCode) -> Self {
        Self {
            to_code: code.0,
            invert: true,
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    let path = canonicalize(&args.path).context(format!("no device {}", args.path.display()))?;
    println!("input device is {}", path.display());

    let mut device = Device::open(path)?;
    println!("{:?}", device.input_id());

    let mut virt_device = make_virt_device(&device)?;
    for path in virt_device.enumerate_dev_nodes_blocking()? {
        println!("virt device available as {}", path?.display());
    }

    let mapping = [
        (
            AbsoluteAxisCode::ABS_X,
            Mapping::from_abs_inv(AbsoluteAxisCode::ABS_X),
        ),
        (
            AbsoluteAxisCode::ABS_Y,
            Mapping::from_abs_inv(AbsoluteAxisCode::ABS_Y),
        ),
        (
            AbsoluteAxisCode::ABS_Z,
            Mapping::from_abs(AbsoluteAxisCode::ABS_RX),
        ),
        (
            AbsoluteAxisCode::ABS_RX,
            Mapping::from_abs(AbsoluteAxisCode::ABS_RY),
        ),
    ]
    .into_iter()
    .collect::<HashMap<_, _>>();

    let abs_infos = abs_infos(&device)?;
    loop {
        for ev in device.fetch_events()? {
            match ev.destructure() {
                Synchronization(..) => continue,
                Key(key_event, key_code, _) => {
                    println!("{:?} {:?}", key_event, key_code)
                }
                AbsoluteAxis(event, code, value) => {
                    if let Some(abs_info) = abs_infos.get(&code) {
                        let pad = " ".repeat(14 * code.0 as usize);
                        println!("{:?} {}{:?} {}", event.event_type(), pad, code, value);
                        let mapping = mapping
                            .get(&code)
                            .copied()
                            .unwrap_or(Mapping::from_abs(code));
                        let value = if mapping.invert {
                            abs_info.minimum() + (abs_info.maximum() - value)
                        } else {
                            value
                        };
                        let virt_ev =
                            *AbsoluteAxisEvent::new(AbsoluteAxisCode(mapping.to_code), value);
                        virt_device.emit(&[virt_ev])?;
                    }
                }
                _ => {
                    println!("other {:?}", ev)
                }
            }
        }
    }
}

fn make_virt_device(device: &Device) -> Result<VirtualDevice> {
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
    let abs_infos = abs_infos(device)?;
    let virt_device = VirtualDeviceBuilder::new()?
        .name(xbox_name)
        .input_id(xbox_id)
        .with_keys(&keys)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_X, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_Y, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_Z, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_RX, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_RY, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_RZ, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_HAT0X, &abs_infos)?)?
        .with_absolute_axis(&abs_setup(AbsoluteAxisCode::ABS_HAT0Y, &abs_infos)?)?
        .build()?;
    Ok(virt_device)
}

fn abs_infos(device: &Device) -> Result<HashMap<AbsoluteAxisCode, AbsInfo>> {
    Ok(device
        .get_absinfo()?
        .map(|(code, i)| {
            (
                code,
                AbsInfo::new(i.value(), i.minimum(), i.maximum(), 0, 0, i.resolution()),
            )
        })
        .collect::<HashMap<_, _>>())
}

fn abs_setup(
    code: AbsoluteAxisCode,
    abs_infos: &HashMap<AbsoluteAxisCode, AbsInfo>,
) -> Result<UinputAbsSetup> {
    let axis_max = 256;
    let default_info = AbsInfo::new(axis_max / 2, 0, axis_max, 0, 0, 1);
    let info = abs_infos.get(&code).cloned().unwrap_or(default_info);
    Ok(UinputAbsSetup::new(code, info))
}
