#include <version.h>

#if KERNEL_VERSION_MAJOR < 3
#include <zephyr.h>
#else
#include <zephyr/kernel.h>
#include <zephyr/device.h>
#endif

/*
 * Export the eeprom device name for the Rust test. The sim-eeprom node moved
 * from /eeprom to /eeprom0 in Zephyr 2.7, and Zephyr 3 dropped the label
 * property from the zephyr,sim-eeprom binding, so derive the name from the
 * devicetree instead of hardcoding it.
 */
const char rust_eeprom_name[] =
#if KERNEL_VERSION_MAJOR < 3
	DT_LABEL(DT_NODELABEL(eeprom0));
#else
	DEVICE_DT_NAME(DT_NODELABEL(eeprom0));
#endif
