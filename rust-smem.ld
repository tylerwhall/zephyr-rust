SECTIONS
{
    data_smem_rust_std_partition_data : { *librust_app.a:*(.data .data.*) }
    data_smem_rust_std_partition_bss : { *librust_app.a:*(.bss .bss.* COMMON COMMON.*) }
} INSERT BEFORE .data;
