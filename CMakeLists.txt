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
    if(CONFIG_CPU_AARCH32_CORTEX_R OR CONFIG_CPU_CORTEX_R)
        if(CONFIG_FPU)
            set(rust_target thumbv7r-zephyr-eabihf)
            set(clang_target thumbv7r-unknown-none-eabihf)
        else()
            set(rust_target thumbv7r-zephyr-eabi)
            set(clang_target thumbv7r-unknown-none-eabi)
        endif()
    elseif(CONFIG_CPU_CORTEX_M4 OR CONFIG_CPU_CORTEX_M7)
        if(CONFIG_FPU)
            set(rust_target thumbv7em-zephyr-eabihf)
            set(clang_target thumbv7em-unknown-none-eabihf)
        else()
            set(rust_target thumbv7em-zephyr-eabi)
            set(clang_target thumbv7em-unknown-none-eabi)
        endif()
    elseif(CONFIG_ARMV8_M_MAINLINE)
        if(CONFIG_FPU)
            set(rust_target thumbv8m.main-zephyr-eabihf)
            set(clang_target thumbv8m.main-unknown-none-eabihf)
        else()
            set(rust_target thumbv8m.main-zephyr-eabi)
            set(clang_target thumbv8m.main-unknown-none-eabi)
        endif()
    else()
        set(rust_target thumbv7m-zephyr-eabi)
        set(clang_target thumbv7m-none-eabi)
    endif()
elseif(${ARCH} STREQUAL "riscv")
    if(CONFIG_64BIT)
        set(rust_target riscv64)
    else()
        set(rust_target riscv32)
    endif()
    # Clang does not know the zephyr target or the ISA suffixes; the base
    # triple is enough for bindgen to process the headers.
    set(clang_target ${rust_target})
    if(CONFIG_RISCV_ISA_RV32E)
        string(APPEND rust_target "e")
    else()
        string(APPEND rust_target "i")
    endif()
    if(CONFIG_RISCV_ISA_EXT_M)
        string(APPEND rust_target "m")
    endif()
    if(CONFIG_RISCV_ISA_EXT_A)
        string(APPEND rust_target "a")
    endif()
    if(CONFIG_RISCV_ISA_EXT_C)
        string(APPEND rust_target "c")
    endif()
    string(APPEND rust_target "-unknown-zephyr-elf")
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
    # Expose the target name as a variable so the ExternalProject_Add
    # DEPENDS below can reference it (stays empty if ZEPHYR_BINDGEN is
    # predefined, in which case there is nothing to build).
    set(zephyr_bindgen_target zephyr_bindgen_target)

    add_custom_command(OUTPUT ${ZEPHYR_BINDGEN}
        WORKING_DIRECTORY ${zephyr_bindgen_src_dir}
        DEPENDS syscall_thunks_target
        COMMAND
              cargo -v build --release --target-dir=${zephyr_bindgen_build_dir}
    )
endif()

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/rust)
set(rust_build_toolchain ${CMAKE_CURRENT_BINARY_DIR}/toolchain)
set(rust_generated_project ${CMAKE_CURRENT_BINARY_DIR}/rust-app)
set(rust_app_build ${CMAKE_CURRENT_BINARY_DIR}/app)
set(rust_staticlib ${rust_app_build}/${rust_target}/release/librust_app.a)
if(NOT DEFINED CARGO_SOURCE_DIR)
    set(CARGO_SOURCE_DIR ${CMAKE_SOURCE_DIR})
endif()

set(rust_project_args "")
if(CONFIG_RUST_ALLOC_POOL)
    list(APPEND rust_project_args --alloc-pool)
endif()
set(rust_build_std std,panic_abort)
if(NOT CONFIG_RUST_STD)
    set(rust_build_std core,alloc)
    list(APPEND rust_project_args --no-std)
endif()
add_custom_command(OUTPUT ${rust_generated_project}/Cargo.toml
  COMMAND ${rust_src_dir}/genproject.sh ${CARGO_SOURCE_DIR} ${rust_generated_project} ${rust_project_args}
  DEPENDS ${rust_src_dir}/genproject.sh
)
add_custom_target(rust_generated_project DEPENDS ${rust_generated_project}/Cargo.toml)

# external_project_cflags comes from the example: zephyr/samples/application_development/external_lib/CMakeLists.txt
zephyr_get_include_directories_for_lang_as_string(       C includes)
zephyr_get_system_include_directories_for_lang_as_string(C system_includes)
zephyr_get_compile_definitions_for_lang_as_string(       C definitions)
# `options` is not included because many of the flags are not supported by clang
# `-imacros ${AUTOCONF_H}` is needed and would have been in `options`
#zephyr_get_compile_options_for_lang_as_string(           C options)

set(external_project_cflags
    "${includes} ${definitions} ${system_includes} -imacros ${AUTOCONF_H} --target=${clang_target}"
)

# The environment the Rust build needs, written to a file that is sourced by
# rust/cargo.sh and ci/clippy.sh, so this is the single place it is defined.
# Generated at configure time; all values are configure-time facts and the
# paths are absolute, so the file is only valid for this build directory.
# Write to the top of the build tree so consumers (ci/clippy.sh) can find
# it without knowing the module's binary dir.
set(rust_env_file ${CMAKE_BINARY_DIR}/rust-env.sh)
set(rust_env_content
    "# Generated by CMake at configure time; do not edit.\n# Sourced by rust/cargo.sh and ci/clippy.sh.\n")
function(rust_env_write var value)
    string(REPLACE "'" "'\\''" rust_env_escaped "${value}")
    string(APPEND rust_env_content "export ${var}='${rust_env_escaped}'\n")
    set(rust_env_content "${rust_env_content}" PARENT_SCOPE)
endfunction()
rust_env_write(RUST_TARGET "${rust_target}")
rust_env_write(TARGET_CFLAGS "${external_project_cflags}")
rust_env_write(RUST_BUILD_TOOLCHAIN "${rust_build_toolchain}")
rust_env_write(RUST_BUILD_STD "${rust_build_std}")
rust_env_write(RUST_TARGET_SPEC "${rust_src_dir}/targets/${rust_target}.json")
rust_env_write(CARGO_MANIFEST "${rust_generated_project}/Cargo.toml")
set(rust_bindings_dir ${CMAKE_CURRENT_BINARY_DIR}/bindings)
rust_env_write(ZEPHYR_RUST_BINDINGS "${rust_bindings_dir}")
# Zephyr version cfgs, consumed by zephyr-core and app crates (e.g.
# tests/eeprom), exported through RUSTFLAGS so they reach every crate in the
# std and app instances (build.rs rustc-cfgs only reach the crate whose
# build script ran). This is the single source of these cfgs.
# Register all version cfgs, including those disabled for this image, for
# Rust 1.80's unexpected_cfgs lint.
set(rust_version_cfgs "")
foreach(cfg zephyr250 zephyr270 zephyr300 zephyr350)
    string(APPEND rust_version_cfgs "--check-cfg=cfg(${cfg}) ")
endforeach()
if(KERNEL_VERSION_NUMBER GREATER_EQUAL 0x20500)
    string(APPEND rust_version_cfgs "--cfg zephyr250 ")
endif()
if(KERNEL_VERSION_NUMBER GREATER_EQUAL 0x20700)
    string(APPEND rust_version_cfgs "--cfg zephyr270 ")
endif()
if(KERNEL_VERSION_NUMBER GREATER_EQUAL 0x30000)
    string(APPEND rust_version_cfgs "--cfg zephyr300 ")
endif()
if(KERNEL_VERSION_NUMBER GREATER_EQUAL 0x30500)
    string(APPEND rust_version_cfgs "--cfg zephyr350 ")
endif()
rust_env_write(RUSTFLAGS "${rust_version_cfgs}")
# Unstable features are required for building std. Also allow in the app
# project for now because they're often needed for low level embedded.
rust_env_write(RUSTC_BOOTSTRAP "1")
# Kconfig values consumed by the Rust build scripts, normalized to y/n.
# (Not CONFIG_THREAD_LOCAL_STORAGE: enabling cfg(tls) does not link, see
# the thread_local! usage in zephyr-core's thread.rs.)
foreach(cfg USERSPACE RUST_ALLOC_POOL RUST_MUTEX_POOL POSIX_CLOCK)
    if(CONFIG_${cfg})
        rust_env_write(CONFIG_${cfg} "y")
    else()
        rust_env_write(CONFIG_${cfg} "n")
    endif()
endforeach()
# file(GENERATE) (not file(WRITE)) so the generator expressions in
# TARGET_CFLAGS are evaluated.
file(GENERATE OUTPUT ${rust_env_file} CONTENT "${rust_env_content}")

# Add the Cargo project only if CONFIG_RUST because this always invokes Cargo.
if (CONFIG_RUST)
    include(ExternalProject)

    # Generate once per Zephyr image, not once per Cargo instance of zephyr-sys.
    # Run after kernel's generated headers on every build; bindgen preserves
    # unchanged outputs so Cargo only recompiles when the bindings change.
    add_custom_target(rust_bindings
        COMMAND ${CMAKE_COMMAND} -E make_directory ${rust_bindings_dir}
        COMMAND sh -c ". '${rust_env_file}' && export OUT_DIR='${rust_bindings_dir}' && exec '${ZEPHYR_BINDGEN}'"
        WORKING_DIRECTORY ${rust_src_dir}/zephyr-sys
        DEPENDS syscall_thunks_target ${zephyr_bindgen_target}
        BYPRODUCTS ${rust_bindings_dir}/bindings.rs ${rust_bindings_dir}/syscalls.rs
        VERBATIM
    )

    ExternalProject_Add(
        rust_project
        PREFIX     ${CMAKE_CURRENT_BINARY_DIR}
        SOURCE_DIR ${rust_src_dir}
        BUILD_IN_SOURCE 1
        BUILD_ALWAYS 1
        DEPENDS rust_bindings rust_generated_project
        CONFIGURE_COMMAND ""
        BUILD_COMMAND
            env "RUST_ENV=${rust_env_file}"
            ./cargo.sh build --release --target-dir=${rust_app_build}
            --manifest-path=${rust_generated_project}/Cargo.toml
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

    zephyr_library_import(rust ${rust_staticlib})
    if(CONFIG_USERSPACE)
        set_property(TARGET zephyr_property_target
                     APPEND PROPERTY COMPILE_OPTIONS
                     "-l" librust_app.a "rust_std_partition")
    endif()
endif()
