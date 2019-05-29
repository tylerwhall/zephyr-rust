# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required(VERSION 3.13.1)

include($ENV{ZEPHYR_BASE}/cmake/app/boilerplate.cmake NO_POLICY_SCOPE)
project(hello_world)

target_sources(app PRIVATE src/main.c)

# Use a clang_target known to clang so headers will be processed correctly with
# bindgen. rust_target may point to a custom json target.
if(${ARCH} STREQUAL "posix" OR ${ARCH} STREQUAL "x86")
    set(rust_target i686-unknown-zephyr)
    set(clang_target i686-unknown-linux-gnu)
elseif(${ARCH} STREQUAL "arm")
    set(rust_target thumbv7m-zephyr-eabi)
    set(clang_target thumbv7m-none-eabi)
else()
    message(FATAL_ERROR "Arch ${ARCH} not supported")
endif()

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
add_dependencies(syscall_thunk offsets_h)

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/zephyr-bindgen)
set(rust_build_dir ${CMAKE_CURRENT_BINARY_DIR}/zephyr-bindgen)
set(zephyr_bindgen ${rust_build_dir}/release/zephyr-bindgen)
add_custom_target(zephyr_bindgen_target DEPENDS ${zephyr_bindgen})

add_custom_command(OUTPUT ${zephyr_bindgen}
    WORKING_DIRECTORY ${rust_src_dir}
    DEPENDS ${rust_syscall_macros_h}
    COMMAND
          cargo -v build --release --target-dir=${rust_build_dir}
)

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/rust-app)
set(rust_build_dir ${CMAKE_CURRENT_BINARY_DIR}/rust-app)
set(rust_staticlib ${rust_build_dir}/${rust_target}/release/librust_app.a)

zephyr_get_include_directories_for_lang_as_string(       C includes)
zephyr_get_compile_definitions_for_lang_as_string(       C definitions)

set(external_project_cflags
    "${includes} ${definitions} -imacros ${AUTOCONF_H}"
)

include(ExternalProject)

ExternalProject_Add(
    rust_project
    PREFIX     ${rust_build_dir}
    SOURCE_DIR ${rust_src_dir}
    BUILD_IN_SOURCE 1
    BUILD_ALWAYS 1
    DEPENDS rust_syscall_macros_h_target zephyr_bindgen_target
    CONFIGURE_COMMAND ""
    BUILD_COMMAND
        env
        "ZEPHYR_BINDGEN=${zephyr_bindgen}"
        "CONFIG_USERSPACE=${CONFIG_USERSPACE}"
        "TARGET_CFLAGS=${external_project_cflags} --target=${clang_target}"
        "SYSROOT=${rust_build_dir}/sysroot"
        "RUST_TARGET_PATH=${rust_src_dir}"
        "CARGO_TARGET_DIR=${rust_build_dir}"
        "RUST_TARGET=${rust_target}"
        ./build.sh
    INSTALL_COMMAND ""
    BUILD_BYPRODUCTS ${rust_staticlib}
)

add_library(rust_lib STATIC IMPORTED)
add_dependencies(rust_lib rust_project)
set_target_properties(rust_lib PROPERTIES IMPORTED_LOCATION ${rust_staticlib})
# This should be a dependency of rust_lib, but adding it as an INTERFACE lib doesn't do anything
target_link_libraries(app PRIVATE syscall_thunk)
target_link_libraries(app PUBLIC rust_lib)
