extern crate zephyr_sys;

use std::convert::TryInto;
use std::ffi::CStr;

use zephyr::device::DeviceSyscalls;
use zephyr::eeprom::Eeprom;

#[no_mangle]
pub extern "C" fn test_main() {
    use zephyr::context::Any as C;

    let eeprom = unsafe {
        let device = C::device_get_binding(CStr::from_bytes_with_nul_unchecked(
            zephyr_sys::raw::DT_ALIAS_EEPROM_0_LABEL,
        ))
        .expect("get eeprom");
        Eeprom::new(device)
    };

    let write = [1, 2, 3, 4];
    let mut read = [0; 4];

    let size = eeprom.size::<C>();
    println!("EEPROM size {}", size);
    // Out of bounds read
    assert_eq!(
        eeprom
            .read::<C>(size.try_into().unwrap(), &mut read)
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );

    eeprom.read::<C>(0, &mut read).expect("read");
    println!("Initial: {:?}", &read);
    eeprom.write::<C>(0, &write).expect("write");
    eeprom.read::<C>(0, &mut read).expect("read");
    println!("After write: {:?}", &read);
    assert_eq!(&read, &write);
}
