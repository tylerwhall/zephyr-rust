$ZEPHYR_SDK_INSTALL_DIR/sysroots/x86_64-pokysdk-linux/usr/bin/qemu-system-i386 \
-m 12 -cpu qemu32,+nx,+pae -device isa-debug-exit,iobase=0xf4,iosize=0x04 \
-no-reboot -nographic -no-acpi -net none -pidfile qemu.pid -serial mon:stdio \
-serial tcp:localhost:4444,server,nowait \
-kernel ./zephyr/zephyr.elf
