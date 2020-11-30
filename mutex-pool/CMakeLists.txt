if(CONFIG_RUST_MUTEX_POOL)

zephyr_library_named(mutex_pool)

set(mutex_pool_c ${CMAKE_CURRENT_BINARY_DIR}/mutex-pool.c)

add_custom_command(OUTPUT ${mutex_pool_c}
    COMMAND
    ${PYTHON_EXECUTABLE}
    ${CMAKE_CURRENT_SOURCE_DIR}/gen-mutex-pool.py ${CONFIG_RUST_MUTEX_POOL_SIZE} > ${mutex_pool_c}
    DEPENDS ${CMAKE_CURRENT_SOURCE_DIR}/gen-mutex-pool.py
)
add_custom_target(mutex_pool_c_target DEPENDS ${mutex_pool_c})

target_sources(mutex_pool PRIVATE ${mutex_pool_c})
add_dependencies(mutex_pool mutex_pool_c_target)

endif()
