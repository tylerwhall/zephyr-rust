# SPDX-License-Identifier: Apache-2.0
set(rust_syscall_macros_h ${ZEPHYR_BINARY_DIR}/include/generated/rust_syscall_macros.h)
add_custom_target(rust_syscall_macros_h_target DEPENDS ${rust_syscall_macros_h})
add_custom_command(OUTPUT ${rust_syscall_macros_h}
  COMMAND
  ${PYTHON_EXECUTABLE}
  ${CMAKE_CURRENT_SOURCE_DIR}/gen_syscall_header.py
  > ${rust_syscall_macros_h}
  DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/gen_syscall_header.py
  )

add_library(syscall_thunk OBJECT syscall_thunk.c)
target_link_libraries(syscall_thunk zephyr_interface)
# High level target that means all headers have been generated
add_dependencies(syscall_thunk offsets_h rust_syscall_macros_h_target)

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/zephyr-bindgen)
set(rust_build_dir ${CMAKE_CURRENT_BINARY_DIR}/zephyr-bindgen )
set(zephyr_bindgen ${rust_build_dir}/release/zephyr-bindgen)
set(zephyr_bindgen ${rust_build_dir}/release/zephyr-bindgen PARENT_SCOPE)
add_custom_target(zephyr_bindgen_target DEPENDS ${zephyr_bindgen})
add_custom_command(OUTPUT ${zephyr_bindgen}
    WORKING_DIRECTORY ${rust_src_dir}
    DEPENDS ${rust_syscall_macros_h}
    COMMAND
          cargo -v build --release --target-dir=${rust_build_dir}
)
