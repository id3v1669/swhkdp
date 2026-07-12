use std::os::unix::io::RawFd;

use evdev::{AttributeSetRef, EventType, RelativeAxisCode};
use nix::ioctl_write_ptr;

// ref: 'struct input_mask' in linux/input.h
#[repr(C)]
struct InputMask {
    event_type: u32,
    codes_size: u32,
    codes_ptr: u64,
}

ioctl_write_ptr!(eviocsmask, b'E', 0x93, InputMask);

pub fn allowed_rel_axes(supported: Option<&AttributeSetRef<RelativeAxisCode>>) -> u16 {
    let has = |code| supported.is_some_and(|axes| axes.contains(code));
    let mut mask = u16::MAX;
    for (low_res, hi_res) in [
        (RelativeAxisCode::REL_WHEEL, RelativeAxisCode::REL_WHEEL_HI_RES),
        (RelativeAxisCode::REL_HWHEEL, RelativeAxisCode::REL_HWHEEL_HI_RES),
    ] {
        let dropped = if has(hi_res) { low_res } else { hi_res };
        mask &= !(1 << dropped.0);
    }
    mask
}

pub fn is_allowed(mask: u16, code: RelativeAxisCode) -> bool {
    match 1u16.checked_shl(code.0.into()) {
        Some(bit) => mask & bit != 0,
        None => true,
    }
}

pub fn apply_kernel_rel_mask(fd: RawFd, mask: u16) -> nix::Result<()> {
    let codes = mask.to_le_bytes();
    let input_mask = InputMask {
        event_type: u32::from(EventType::RELATIVE.0),
        codes_size: codes.len() as u32,
        codes_ptr: codes.as_ptr() as u64,
    };
    unsafe { eviocsmask(fd, &input_mask) }.map(drop)
}
