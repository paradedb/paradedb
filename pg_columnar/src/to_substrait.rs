void DuckDBToSubstrait::TransformTableScanToSubstrait(LogicalGet &dget, substrait::ReadRel *sget) {
    auto &table_scan_bind_data = dget.bind_data->Cast<TableScanBindData>();
    auto &table = table_scan_bind_data.table;
    sget->mutable_named_table()->add_names(table.name);
    auto base_schema = new ::substrait::NamedStruct();
    auto type_info = new substrait::Type_Struct();
    type_info->set_nullability(substrait::Type_Nullability_NULLABILITY_REQUIRED);
    auto not_null_constraint = GetNotNullConstraintCol(table);
    for (idx_t i = 0; i < dget.names.size(); i++) {
        auto cur_type = dget.returned_types[i];
        if (cur_type.id() == LogicalTypeId::STRUCT) {
            throw std::runtime_error("Structs are not yet accepted in table scans");
        }
        base_schema->add_names(dget.names[i]);
        auto column_statistics = dget.function.statistics(context, &table_scan_bind_data, i);
        bool not_null = not_null_constraint.find(i) != not_null_constraint.end();
        auto new_type = type_info->add_types();
        *new_type = DuckToSubstraitType(cur_type, column_statistics.get(), not_null);
    }
    base_schema->set_allocated_struct_(type_info);
    sget->set_allocated_base_schema(base_schema);
}