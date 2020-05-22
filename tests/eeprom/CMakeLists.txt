# SPDX-License-Identifier: Apache-2.0

cmake_minimum_required(VERSION 3.13.1)

# For this example, add ZEPHYR_RUST as a zephyr module. Not needed if added in
# your environment or managed by west.
get_filename_component(ZEPHYR_RUST ${CMAKE_CURRENT_SOURCE_DIR}/../.. ABSOLUTE)
list(APPEND ZEPHYR_EXTRA_MODULES ${ZEPHYR_RUST})

include($ENV{ZEPHYR_BASE}/cmake/app/boilerplate.cmake NO_POLICY_SCOPE)
project(eeprom_api)

target_sources(app PRIVATE ./src/main.c)
