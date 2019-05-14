# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required(VERSION 3.13.1)

include($ENV{ZEPHYR_BASE}/cmake/app/boilerplate.cmake NO_POLICY_SCOPE)
project(hello_world)

target_sources(app PRIVATE src/main.c)

if(${ARCH} STREQUAL "posix" OR ${ARCH} STREQUAL "x86")
    set(rust_target i686-unknown-linux-gnu)
elseif(${ARCH} STREQUAL "arm")
    set(rust_target thumbv7m-none-eabi)
else()
    message(FATAL_ERROR "Arch ${ARCH} not supported")
endif()

set(rust_src_dir ${CMAKE_CURRENT_SOURCE_DIR}/rust-app)
set(rust_build_dir ${CMAKE_CURRENT_BINARY_DIR}/rust-app)
set(rust_staticlib ${rust_build_dir}/${rust_target}/release/librust_app.a)

include(ExternalProject)

ExternalProject_Add(
  rust_project
  PREFIX     ${rust_build_dir}
  SOURCE_DIR ${rust_src_dir}
  BUILD_IN_SOURCE 1
  BUILD_ALWAYS 1
  CONFIGURE_COMMAND ""
  BUILD_COMMAND
  cargo +nightly build --target=${rust_target} --target-dir=${rust_build_dir} --release
  INSTALL_COMMAND ""
  BUILD_BYPRODUCTS ${rust_staticlib}
)

add_library(rust_lib STATIC IMPORTED GLOBAL)
add_dependencies(
    rust_lib
    rust_project
)
set_target_properties(rust_lib PROPERTIES IMPORTED_LOCATION ${rust_staticlib})
set_directory_properties(PROPERTIES ADDITIONAL_MAKE_CLEAN_FILES ${rust_build_dir})

target_link_libraries(app PUBLIC rust_lib)
