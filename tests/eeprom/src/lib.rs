extern crate zephyr_sys;

use std::convert::TryInto;
use std::ffi::CStr;
use std::os::raw::c_char;

use zephyr::device::DeviceSyscalls;
use zephyr::eeprom::Eeprom;

extern "C" {
    static rust_eeprom_name: [c_char; 0];
}

#[no_mangle]
pub extern "C" fn test_main() {
    use zephyr::context::Any as C;

    let eeprom = unsafe {
        let device = unsafe { CStr::from_ptr(rust_eeprom_name.as_ptr()) };
        let device = C::device_get_binding(device).expect("get eeprom");
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
