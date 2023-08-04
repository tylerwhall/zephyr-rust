# SPDX-License-Identifier: Apache-2.0
cmake_minimum_required(VERSION 3.8.2)

add_subdirectory(uart-buffered)
add_subdirectory(mutex-pool)

# Use a clang_target known to clang so headers will be processed correctly with
# bindgen. rust_target may point to a custom json target.
if(${ARCH} STREQUAL "posix" OR ${ARCH} STREQUAL "x86")
    set(rust_target i686-unknown-zephyr)
    set(clang_target i686-unknown-linux-gnu)
elseif(${ARCH} STREQUAL "arm")
    if(CONFIG_CPU_CORTEX_R)
        if(CONFIG_FPU)
            set(rust_target armv7r-zephyr-eabihf)
            set(clang_target armv7r-unknown-none-eabihf)
        else()
            set(rust_target armv7r-zephyr-eabi)
            set(clang_target armv7r-unknown-none-eabi)
        endif()
    elseif(CONFIG_CPU_CORTEX_M4 OR CONFIG_CPU_CORTEX_M7)
        if(CONFIG_FPU)
            set(rust_target thumbv7em-zephyr-eabihf)
            set(clang_target thumbv7em-unknown-none-eabihf)
        else()
            set(rust_target thumbv7em-zephyr-eabi)
            set(clang_target thumbv7em-unknown-none-eabi)
        endif()
    else()
        set(rust_target thumbv7m-zephyr-eabi)
        set(clang_target thumbv7m-none-eabi)
    endif()
else()
    message(FATAL_ERROR "Arch ${ARCH} not supported")
endif()

message("Rust  target: ${rust_target}")
message("Clang target: ${clang_target}")

set(all_syscalls ${ZEPHYR_BINARY_DIR}/include/generated/all_syscalls.h)
set(syscall_thunks ${ZEPHYR_BINARY_DIR}/include/generated/syscall_thunks.c)
set(syscalls_json  ${ZEPHYR_BINARY_DIR}/misc/generated/syscalls.json)
add_custom_command(OUTPUT ${all_syscalls} ${syscall_thunks}
  # Also, some files are written to include/generated/syscalls/
  COMMAND
  ${PYTHON_EXECUTABLE}
  ${CMAKE_CURRENT_SOURCE_DIR}/scripts/gen_syscalls.py
  --json-file        ${syscalls_json}                     # Read this file
  --thunks           ${syscall_thunks}
  --all-syscalls     ${all_syscalls}
  WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
  DEPENDS ${syscalls_json}
          ${CMAKE_CURRENT_SOURCE_DIR}/scripts/gen_syscalls.py
  )
add_custom_target(syscall_thunks_target DEPENDS
        ${all_syscalls} ${syscall_thunks} ${SYSCALL_LIST_H_TARGET} kernel)

if(NOT ZEPHYR_BINDGEN)
    set(zephyr_bindgen_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/zephyr-bindgen)
    set(zephyr_bindgen_build_dir ${CMAKE_CURRENT_BINARY_DIR}/zephyr-bindgen)
    set(ZEPHYR_BINDGEN ${zephyr_bindgen_build_dir}/release/zephyr-bindgen)
    add_custom_target(zephyr_bindgen_target DEPENDS ${ZEPHYR_BINDGEN})
    set(zephyr_bindgen_target zephyr_bindgen_target)

    add_custom_command(OUTPUT ${ZEPHYR_BINDGEN}
        WORKING_DIRECTORY ${zephyr_bindgen_src_dir}
        DEPENDS syscall_thunks_target
        COMMAND
              cargo -v build --release --target-dir=${zephyr_bindgen_build_dir}
    )
endif()

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/rust)
set(rust_sysroot ${CMAKE_CURRENT_BINARY_DIR}/sysroot)
set(rust_sysroot_build ${CMAKE_CURRENT_BINARY_DIR}/sysroot-build)
set(rust_generated_project ${CMAKE_CURRENT_BINARY_DIR}/rust-app)
set(rust_app_build ${CMAKE_CURRENT_BINARY_DIR}/app)
set(rust_staticlib ${rust_app_build}/${rust_target}/release/librust_app.a)
if(NOT DEFINED CARGO_SOURCE_DIR)
    set(CARGO_SOURCE_DIR ${CMAKE_SOURCE_DIR})
endif()

add_custom_command(OUTPUT ${rust_generated_project}/Cargo.toml
  COMMAND ${rust_src_dir}/genproject.sh ${CARGO_SOURCE_DIR} ${rust_generated_project}
  DEPENDS ${rust_src_dir}/genproject.sh
)
add_custom_target(rust_generated_project DEPENDS ${rust_generated_project}/Cargo.toml)

zephyr_get_include_directories_for_lang_as_string(       C includes)
zephyr_get_compile_definitions_for_lang_as_string(       C definitions)

set(external_project_cflags
    "${includes} ${definitions} -imacros ${AUTOCONF_H}"
)

# Add the Cargo project only if CONFIG_RUST because this will alawys invoke Cargo.
if (CONFIG_RUST)
    include(ExternalProject)

    ExternalProject_Add(
        rust_project
        PREFIX     ${CMAKE_CURRENT_BINARY_DIR}
        SOURCE_DIR ${rust_src_dir}
        BUILD_IN_SOURCE 1
        BUILD_ALWAYS 1
        DEPENDS syscall_thunks_target ${zephyr_bindgen_target} rust_generated_project
        CONFIGURE_COMMAND ""
        BUILD_COMMAND
            env
            "ZEPHYR_BINDGEN=${ZEPHYR_BINDGEN}"
            "ZEPHYR_KERNEL_VERSION_NUM=${KERNEL_VERSION_NUMBER}"
            "CONFIG_USERSPACE=${CONFIG_USERSPACE}"
            "CONFIG_RUST_ALLOC_POOL=${CONFIG_RUST_ALLOC_POOL}"
            "CONFIG_RUST_MUTEX_POOL=${CONFIG_RUST_MUTEX_POOL}"
            "CONFIG_POSIX_CLOCK=${CONFIG_POSIX_CLOCK}"
            "TARGET_CFLAGS=${external_project_cflags} --target=${clang_target}"
            "SYSROOT=${rust_sysroot}"
            "SYSROOT_BUILD=${rust_sysroot_build}"
            "APP_BUILD=${rust_app_build}"
            "RUST_TARGET=${rust_target}"
            "RUST_TARGET_SPEC=${rust_src_dir}/${rust_target}.json"
            "CARGO_MANIFEST=${rust_generated_project}/Cargo.toml"
            ./build.sh
        INSTALL_COMMAND ""
        BUILD_BYPRODUCTS ${rust_staticlib}
    )

    # A regular Zephyr C library for our C code
    zephyr_library_named(rust_c)
    set(thunk_sources syscall-thunk-any.c)
    if(CONFIG_USERSPACE)
        set(thunk_sources ${thunk_sources} syscall-thunk-kernel.c syscall-thunk-user.c)
    endif()
    target_sources(rust_c PRIVATE ${thunk_sources} rust-smem.c abort.c)
    if(DEFINED syscall_thunk_cflags)
        set_source_files_properties(${thunk_sources} PROPERTIES COMPILE_FLAGS "${syscall_thunk_cflags}")
    endif()
    add_dependencies(rust_c syscall_thunks_target)

    # Compatibility for Zephyr 1.14
    if (NOT COMMAND zephyr_library_import)
        # Add the imported library 'library_name', located at 'library_path' to the
        # global list of Zephyr CMake libraries.
        function(zephyr_library_import library_name library_path)
          add_library(${library_name} STATIC IMPORTED GLOBAL)
          set_target_properties(${library_name}
            PROPERTIES IMPORTED_LOCATION
            ${library_path}
            )
          zephyr_append_cmake_library(${library_name})
        endfunction()
    endif()

    zephyr_library_import(rust ${rust_staticlib})
    if(CONFIG_USERSPACE)
        set_property(TARGET zephyr_property_target
                     APPEND PROPERTY COMPILE_OPTIONS
                     "-l" librust_app.a "rust_std_partition")
    endif()
endif()
