extern crate zephyr_sys;

use std::convert::TryInto;
use std::ffi::CStr;

use zephyr::device::DeviceSyscalls;
use zephyr::eeprom::Eeprom;

// The eeprom device name is version dependent: the node moved from /eeprom
// to /eeprom0 in Zephyr 2.7, and Zephyr 3 dropped the label property from
// the zephyr,sim-eeprom binding, so DEVICE_DT_NAME falls back to the node
// full name. The devicetree-generated macros bind directly, so import the
// one that exists for the target Zephyr version.
#[cfg(not(zephyr250))]
use zephyr_sys::raw::DT_N_S_eeprom_P_label as eeprom_name;
#[cfg(all(zephyr250, not(zephyr300)))]
use zephyr_sys::raw::DT_N_S_eeprom0_P_label as eeprom_name;
#[cfg(zephyr300)]
use zephyr_sys::raw::DT_N_S_eeprom0_FULL_NAME as eeprom_name;

#[no_mangle]
pub extern "C" fn test_main() {
    use zephyr::context::Any as C;

    let eeprom = unsafe {
        let device =
            C::device_get_binding(CStr::from_bytes_with_nul_unchecked(eeprom_name))
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
